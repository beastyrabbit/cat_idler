//! Axum server shell for the Cat Colony simulation, porting the transport around
//! `server/game.ts:workerTick` and `app/api/game/actions/route.ts`.

use std::{
    collections::{BTreeMap, BTreeSet},
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
use cat_protocol::{
    ActionResult, ClientAction, VillageCapabilities, VillageKind as ProtocolVillageKind,
    WorldSnapshot,
};
use cat_sim::{
    actions::{ActionCtx, apply_action, build_snapshot},
    world_tick::{
        TilePos, VillageKind, VillageScale, WorldState, found_global_colony, new_world, world_tick,
    },
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
    /// Non-wire ownership directory used to project the shared tick snapshot for
    /// each socket. Keeping this separate makes it impossible for serialization
    /// to accidentally expose stable owner identifiers.
    village_directory: Arc<RwLock<BTreeMap<String, VillageDirectoryEntry>>>,
    online_count: Arc<AtomicU32>,
    rate_limiter: Arc<Mutex<RateLimiter>>,
    session_secret: Arc<String>,
    allow_test_actions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VillageDirectoryEntry {
    name: String,
    anchor: TilePos,
    kind: VillageKind,
    scale: VillageScale,
    owner_player_id: Option<String>,
    known_village_ids: BTreeSet<String>,
}

fn village_directory(world: &WorldState) -> BTreeMap<String, VillageDirectoryEntry> {
    world
        .colonies
        .iter()
        .map(|colony| {
            (
                colony.id.clone(),
                VillageDirectoryEntry {
                    name: colony.name.clone(),
                    anchor: colony.anchor,
                    kind: colony.kind,
                    scale: colony.scale,
                    owner_player_id: colony.owner_player_id.clone(),
                    known_village_ids: colony.known_village_ids.clone(),
                },
            )
        })
        .collect()
}

fn global_village_id(directory: &BTreeMap<String, VillageDirectoryEntry>) -> String {
    directory
        .iter()
        .find_map(|(id, entry)| (entry.kind == VillageKind::Global).then(|| id.clone()))
        .unwrap_or_else(|| STARTER_COLONY_ID.to_owned())
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
    world.colonies.push(found_global_colony(
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
    let mut world = load_world(&conn)?.unwrap_or_else(|| starter_world(now_ms));
    if world.colonies.is_empty() {
        world = starter_world(now_ms);
    }
    if world
        .colonies
        .iter()
        .filter(|colony| colony.kind == VillageKind::Global)
        .count()
        != 1
    {
        return Err(rusqlite::Error::InvalidParameterName(
            "loaded world must contain exactly one global village".to_owned(),
        ));
    }
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
    let village_directory = village_directory(&world);

    AppState {
        world: Arc::new(Mutex::new(world)),
        db: Arc::new(Mutex::new(conn)),
        completed_snapshot: Arc::new(RwLock::new(completed_snapshot)),
        snapshots,
        village_directory: Arc::new(RwLock::new(village_directory)),
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
    village_directory: BTreeMap<String, VillageDirectoryEntry>,
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
        let village_directory = village_directory(&world);
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
            village_directory,
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
    *state.village_directory.write().await = completed.village_directory;
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
    let directory = state.village_directory.read().await;
    let global_id = global_village_id(&directory);
    drop(directory);
    let mut connection = ConnectionContext::new(format!("ws-{connection_id}"), global_id);
    let online_count = state.online_count.fetch_add(1, Ordering::SeqCst) + 1;
    let mut snapshots = state.snapshots.subscribe();

    if send_current_snapshot(&mut socket, &state, online_count, &connection)
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
                        let directory = state.village_directory.read().await;
                        let snapshot = project_snapshot(
                            snapshot,
                            &directory,
                            connection.identity.as_ref(),
                            &connection.colony_id,
                        );
                        drop(directory);
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
    connection: &ConnectionContext,
) -> Result<(), axum::Error> {
    let snapshot = current_snapshot(state, online_count, connection).await;
    send_snapshot(socket, &snapshot).await
}

async fn current_snapshot(
    state: &AppState,
    online_count: u32,
    connection: &ConnectionContext,
) -> WorldSnapshot {
    let mut snapshot = state.completed_snapshot.read().await.clone();
    snapshot.online_count = online_count;
    let directory = state.village_directory.read().await;
    project_snapshot(
        snapshot,
        &directory,
        connection.identity.as_ref(),
        &connection.colony_id,
    )
}

/// Produce the only snapshot shape that may cross a socket. Undiscovered
/// personal villages are removed server-side, so hiding selector rows in the
/// client can never become a privacy boundary.
fn project_snapshot(
    mut snapshot: WorldSnapshot,
    directory: &BTreeMap<String, VillageDirectoryEntry>,
    identity: Option<&SignedSession>,
    requested_colony_id: &str,
) -> WorldSnapshot {
    snapshot.known_villages.clear();
    snapshot.colonies.retain(|colony| {
        directory.get(&colony.id).is_some_and(|entry| {
            entry.kind == VillageKind::Global
                || identity.is_some_and(|identity| {
                    entry.owner_player_id.as_deref() == Some(identity.player_id.as_str())
                })
        })
    });
    for colony in &mut snapshot.colonies {
        let Some(entry) = directory.get(&colony.id) else {
            continue;
        };
        let is_owner = identity.is_some_and(|identity| {
            entry.owner_player_id.as_deref() == Some(identity.player_id.as_str())
        });
        colony.kind = match entry.kind {
            VillageKind::Global => ProtocolVillageKind::Global,
            VillageKind::Personal => ProtocolVillageKind::Personal,
        };
        colony.capabilities = VillageCapabilities {
            can_view: true,
            can_control: identity.is_some() && (entry.kind == VillageKind::Global || is_owner),
            is_owner,
        };
        project_reported_stock(colony);
    }

    let controlled_ids = snapshot
        .colonies
        .iter()
        .filter(|colony| colony.capabilities.can_control)
        .map(|colony| colony.id.clone())
        .collect::<BTreeSet<_>>();
    snapshot.village_trade_offers.retain(|offer| {
        identity.is_some()
            && (controlled_ids.contains(&offer.from_colony_id)
                || controlled_ids.contains(&offer.to_colony_id))
    });

    let selected = snapshot
        .colonies
        .iter()
        .any(|colony| colony.id == requested_colony_id)
        .then(|| requested_colony_id.to_owned())
        .or_else(|| {
            snapshot
                .colonies
                .iter()
                .find(|colony| colony.kind == ProtocolVillageKind::Global)
                .map(|colony| colony.id.clone())
        })
        .or_else(|| snapshot.colonies.first().map(|colony| colony.id.clone()));
    snapshot.selected_colony_id.clone_from(&selected);
    if let Some(selected_id) = selected.as_deref()
        && let Some(selected_entry) = directory.get(selected_id)
    {
        snapshot.known_villages = selected_entry
            .known_village_ids
            .iter()
            .filter_map(|village_id| {
                let entry = directory.get(village_id)?;
                let is_owner = identity.is_some_and(|identity| {
                    entry.owner_player_id.as_deref() == Some(identity.player_id.as_str())
                });
                Some(cat_protocol::VillageSummary {
                    id: village_id.clone(),
                    name: entry.name.clone(),
                    kind: match entry.kind {
                        VillageKind::Global => ProtocolVillageKind::Global,
                        VillageKind::Personal => ProtocolVillageKind::Personal,
                    },
                    scale: match entry.scale {
                        VillageScale::Personal => cat_protocol::VillageScale::Personal,
                        VillageScale::Communal => cat_protocol::VillageScale::Communal,
                    },
                    anchor: cat_protocol::TilePoint {
                        x: entry.anchor.x,
                        y: entry.anchor.y,
                    },
                    capabilities: VillageCapabilities {
                        can_view: entry.kind == VillageKind::Global || is_owner,
                        can_control: identity.is_some()
                            && (entry.kind == VillageKind::Global || is_owner),
                        is_owner,
                    },
                })
            })
            .collect();
    }
    if let Some(selected) = selected {
        prioritize_colony(snapshot, &selected)
    } else {
        snapshot
    }
}

/// Remove authoritative stock from the only snapshot shape allowed to cross a socket.
/// The completed snapshot cache deliberately remains exact for trusted server work. A socket
/// receives only the Accountant's aggregate and per-pile reports. Divine blessings are not
/// physical stockpile goods, so their spendable balance remains exact. Equality attestations
/// are cleared as well: even a boolean "still accurate" would reveal an unseen stock change.
fn project_reported_stock(colony: &mut cat_protocol::ColonySnapshot) {
    let blessings = colony.resources.blessings;
    let Some(ledger) = colony.stock_ledger.as_mut() else {
        // `cat-sim::build_snapshot` always emits a ledger. Keeping this legacy branch
        // conservative prevents a future/foreign canonical snapshot from leaking via a
        // missing report.
        colony.resources = cat_protocol::ResourceAmounts {
            blessings,
            ..cat_protocol::ResourceAmounts::default()
        };
        colony.threat.weapons = 0.0;
        colony.threat.armor = 0.0;
        for pile in &mut colony.stockpiles {
            pile.contents = pile
                .report
                .as_ref()
                .map_or_else(cat_protocol::ResourceAmounts::default, |report| {
                    report.reported
                });
            if let Some(report) = &mut pile.report {
                report.accurate = false;
            }
        }
        return;
    };

    colony.resources = ledger.reported;
    colony.resources.blessings = blessings;
    colony.threat.weapons = ledger.reported.weapons;
    colony.threat.armor = ledger.reported.armor;
    ledger.accurate = false;
    for pile in &mut colony.stockpiles {
        pile.contents = pile
            .report
            .as_ref()
            .map_or_else(cat_protocol::ResourceAmounts::default, |report| {
                report.reported
            });
        if let Some(report) = &mut pile.report {
            report.accurate = false;
        }
    }
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
    fn new(limiter_fallback: String, global_colony_id: String) -> Self {
        Self {
            limiter_fallback,
            identity: None,
            colony_id: global_colony_id,
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
            colony_id: None,
        })
    }

    fn fail(message: impl Into<String>) -> Self {
        Self::from_result(ActionResult {
            ok: false,
            message: Some(message.into()),
            colony_id: None,
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
        if let Some(colony_id) = &self.result.colony_id {
            object.insert(
                "colonyId".to_owned(),
                serde_json::Value::String(colony_id.clone()),
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
            let directory = state.village_directory.read().await;
            connection.colony_id = global_village_id(&directory);
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
                if let Some(colony_id) = &result.colony_id {
                    connection.colony_id.clone_from(colony_id);
                }
            }
            ClientAction::JoinVillage { colony_id, .. } => {
                connection.colony_id.clone_from(colony_id);
            }
            _ => {}
        }
    }
    let refreshed_directory =
        matches!(action, ClientAction::FoundVillage { .. }).then(|| village_directory(&world));
    let refreshed_snapshot = result
        .ok
        .then(|| build_snapshot(&world, now, state.online_count.load(Ordering::SeqCst)));
    drop(world);
    if let Some(directory) = refreshed_directory {
        *state.village_directory.write().await = directory;
    }
    if let Some(snapshot) = refreshed_snapshot {
        *state.completed_snapshot.write().await = snapshot.clone();
        let _ = state.snapshots.send(snapshot);
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
        | ClientAction::ResearchNode {
            session_id, sig, ..
        }
        | ClientAction::OfferTithe {
            session_id, sig, ..
        }
        | ClientAction::OfferMaterials {
            session_id, sig, ..
        }
        | ClientAction::HaulGatherSpot {
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
        | ClientAction::RepairItem {
            session_id, sig, ..
        }
        | ClientAction::BuyResource {
            session_id, sig, ..
        }
        | ClientAction::BoostCat {
            session_id, sig, ..
        }
        | ClientAction::SetCatLaborPreference {
            session_id, sig, ..
        }
        | ClientAction::EditProductionQueue {
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
        | ClientAction::DesignateFishingSpot {
            session_id, sig, ..
        }
        | ClientAction::RemoveGatherSpot {
            session_id, sig, ..
        }
        | ClientAction::OfferVillageTrade {
            session_id, sig, ..
        }
        | ClientAction::AcceptVillageTrade {
            session_id, sig, ..
        }
        | ClientAction::CancelVillageTrade {
            session_id, sig, ..
        } => ActionAuthentication::Signed { session_id, sig },
        ClientAction::FoundVillage {
            session_id, sig, ..
        }
        | ClientAction::JoinVillage {
            session_id, sig, ..
        } => match sig {
            Some(sig) => ActionAuthentication::Signed { session_id, sig },
            None => ActionAuthentication::SessionBound { session_id },
        },
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
    use cat_sim::world_tick::found_colony;
    use std::{collections::BTreeSet, fs, path::PathBuf, time::Duration};
    use tower::ServiceExt;

    static NEXT_STATIC_FIXTURE_ID: AtomicU64 = AtomicU64::new(1);
    static NEXT_DATABASE_FIXTURE_ID: AtomicU64 = AtomicU64::new(1);

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
            current_snapshot(
                &state,
                7,
                &ConnectionContext::new("slow-tick-test".to_owned(), STARTER_COLONY_ID.to_owned()),
            ),
        )
        .await
        .expect("websocket initial snapshot reads the completed cache");
        assert_eq!(initial.now, startup_snapshot.now);
        assert_eq!(initial.online_count, 7);
        assert_eq!(initial.colonies[0].id, startup_snapshot.colonies[0].id);
        assert!(!initial.colonies[0].capabilities.can_control);

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
        let owner = signed_session("owner-session".to_owned(), "test-session-secret");
        let mut beta_colony = found_colony(WORLD_SEED, "beta", 1_000_000, 2);
        beta_colony.kind = VillageKind::Personal;
        beta_colony.owner_player_id = Some(owner.player_id.clone());
        world.colonies.push(beta_colony);
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        persistence::init_schema(&conn).expect("init in-memory schema");
        let state = build_state_from_world(
            world,
            conn,
            "test-session-secret".to_owned(),
            false,
            1_000_000,
        );

        let mut owner_connection =
            ConnectionContext::new("owner".to_owned(), STARTER_COLONY_ID.to_owned());
        owner_connection.identity = Some(owner);
        owner_connection.colony_id = "beta".to_owned();
        let beta = current_snapshot(&state, 1, &owner_connection).await;
        assert_eq!(beta.colonies[0].id, "beta");
        let canonical = state.completed_snapshot.read().await;
        assert_eq!(canonical.colonies[0].id, STARTER_COLONY_ID);
        assert_eq!(canonical.online_count, 0);
    }

    #[test]
    fn authenticated_owner_wire_never_bypasses_vacant_or_blocked_accountant_reports() {
        use cat_sim::{
            entities::Resources,
            ledger::{AccountingPhase, AccountingRound, PileReport, StockLedger},
            stockpiles::{ResourceKind as SimResourceKind, Stockpile},
            zones::ZoneRect,
        };

        for blocked in [false, true] {
            let signed =
                signed_session(format!("accountant-wire-{blocked}"), "test-session-secret");
            let mut world = new_world(WORLD_SEED);
            let mut colony =
                found_colony(WORLD_SEED, "private-books", 1_000_000, STARTER_COLONY_SEED);
            colony.kind = VillageKind::Personal;
            colony.owner_player_id = Some(signed.player_id.clone());

            let pile = colony
                .stockpiles
                .iter_mut()
                .find(|pile| pile.is_general_storehouse())
                .expect("founding storehouse");
            pile.contents.food = 91_234.5;
            pile.contents.water = 82_234.5;
            pile.contents.weapons = 17_234.5;
            pile.contents.armor = 18_234.5;
            let pile_id = pile.id.clone();
            colony.resources.food = 91_234.5;
            colony.resources.water = 82_234.5;
            colony.resources.weapons = 17_234.5;
            colony.resources.armor = 18_234.5;
            colony.resources.materials = 66_234.5;
            colony.global_upgrade_points = 7.0;
            colony.stockpiles.push(Stockpile {
                id: "uncounted-cache".to_owned(),
                rect: ZoneRect {
                    x1: colony.anchor.x + 7,
                    y1: colony.anchor.y,
                    x2: colony.anchor.x + 7,
                    y2: colony.anchor.y,
                },
                accepts: [SimResourceKind::Materials].into_iter().collect(),
                contents: Resources {
                    materials: 66_234.5,
                    ..Resources::default()
                },
            });

            let reported = Resources {
                food: 13.0,
                water: 12.0,
                weapons: 3.0,
                armor: 4.0,
                ..Resources::default()
            };
            colony.stock_ledger = StockLedger {
                reported: reported.clone(),
                last_counted: 900_000,
                pile_reports: [(
                    pile_id.clone(),
                    PileReport {
                        reported,
                        last_counted: 900_000,
                    },
                )]
                .into_iter()
                .collect(),
                active_round: blocked.then(|| AccountingRound {
                    worker_id: "bookkeeper".to_owned(),
                    tent_id: "accounting-tent".to_owned(),
                    phase: AccountingPhase::WaitingAtTent,
                    unreachable_stockpile_ids: vec!["uncounted-cache".to_owned()],
                    ..AccountingRound::default()
                }),
                ..StockLedger::default()
            };
            world.colonies.push(colony);

            let canonical = build_snapshot(&world, 1_000_000, 1);
            assert_eq!(canonical.colonies[0].resources.food, 91_234.5);
            assert_eq!(canonical.colonies[0].stockpiles[0].contents.food, 91_234.5);
            assert_eq!(
                canonical.colonies[0]
                    .stockpiles
                    .iter()
                    .find(|pile| pile.id == "uncounted-cache")
                    .expect("canonical uncounted pile")
                    .contents
                    .materials,
                66_234.5
            );

            let projected = project_snapshot(
                canonical.clone(),
                &village_directory(&world),
                Some(&signed),
                "private-books",
            );
            let player = &projected.colonies[0];
            assert!(player.capabilities.is_owner);
            assert_eq!(player.resources.food, 13.0);
            assert_eq!(player.resources.water, 12.0);
            assert_eq!(player.resources.materials, 0.0);
            assert_eq!(player.resources.blessings, 7.0);
            assert_eq!(player.threat.weapons, 3.0);
            assert_eq!(player.threat.armor, 4.0);
            assert_eq!(player.stockpiles[0].contents.food, 13.0);
            assert_eq!(player.stockpiles[0].contents.water, 12.0);
            let uncounted = player
                .stockpiles
                .iter()
                .find(|pile| pile.id == "uncounted-cache")
                .expect("projected uncounted pile");
            assert_eq!(uncounted.contents, cat_protocol::ResourceAmounts::default());
            assert!(uncounted.report.is_none());

            let json = serde_json::to_value(&projected).expect("owner websocket payload");
            let owner = &json["colonies"][0];
            assert!(owner["stockLedger"].get("accurate").is_none());
            assert!(owner["stockpiles"][0]["report"].get("accurate").is_none());
            let wire = serde_json::to_string(&json).expect("owner websocket JSON");
            for sentinel in ["91234.5", "82234.5", "66234.5", "17234.5", "18234.5"] {
                assert!(
                    !wire.contains(sentinel),
                    "authoritative sentinel {sentinel} crossed the player wire"
                );
            }
            assert_eq!(canonical.colonies[0].resources.materials, 66_234.5);
            assert_eq!(canonical.colonies[0].stockpiles[0].contents.food, 91_234.5);
        }
    }

    #[tokio::test]
    async fn every_owner_socket_emission_path_applies_the_accountant_projection() {
        let secret = "guided-campaign-secret";
        let signed = signed_session("accountant-paths".to_owned(), secret);
        let private_id = "accountant-private";
        let mut world = starter_world(1_000_000);
        let mut private = found_colony(WORLD_SEED, private_id, 1_000_000, STARTER_COLONY_SEED + 1);
        private.kind = VillageKind::Personal;
        private.owner_player_id = Some(signed.player_id.clone());
        private.global_upgrade_points = 7.0;
        let cat_id = private.cats[0].id.clone();
        private.resources.food = 91_234.5;
        private.resources.weapons = 17_234.5;
        let storehouse = private
            .stockpiles
            .iter_mut()
            .find(|pile| pile.is_general_storehouse())
            .expect("private storehouse");
        storehouse.contents.food = 91_234.5;
        storehouse.contents.weapons = 17_234.5;
        world.colonies.push(private);

        let state = build_test_state_from_world(world, 1_000_000);
        let mut connection =
            ConnectionContext::new("accountant-owner".to_owned(), private_id.to_owned());
        connection.identity = Some(signed.clone());

        let assert_safe_wire = |snapshot: &WorldSnapshot, phase: &str| {
            let player = &snapshot.colonies[0];
            assert_eq!(player.id, private_id, "{phase} selected village");
            assert!(player.capabilities.is_owner, "{phase} owner capability");
            assert_eq!(player.resources.food, 50.0, "{phase} aggregate report");
            assert_eq!(player.resources.blessings, 7.0, "{phase} exact blessings");
            assert_eq!(player.threat.weapons, 0.0, "{phase} defense duplicate");
            assert_eq!(
                player
                    .stockpiles
                    .iter()
                    .find(|pile| pile.id == cat_sim::stockpiles::GENERAL_STOREHOUSE_ID)
                    .expect("wire storehouse")
                    .contents
                    .food,
                50.0,
                "{phase} pile report"
            );
            let json = serde_json::to_value(snapshot).expect("socket JSON");
            let owner = &json["colonies"][0];
            assert!(
                owner["stockLedger"].get("accurate").is_none(),
                "{phase} aggregate equality oracle"
            );
            for pile in owner["stockpiles"].as_array().expect("stockpiles") {
                if let Some(report) = pile.get("report") {
                    assert!(
                        report.get("accurate").is_none(),
                        "{phase} pile equality oracle"
                    );
                }
            }
            let wire = serde_json::to_string(&json).expect("socket text");
            for sentinel in ["91234.5", "17234.5", "73456.25"] {
                assert!(
                    !wire.contains(sentinel),
                    "{phase} leaked authoritative sentinel {sentinel}"
                );
            }
        };

        let initial = current_snapshot(&state, 1, &connection).await;
        assert_safe_wire(&initial, "initial cache");
        assert_eq!(
            state.completed_snapshot.read().await.colonies[1]
                .resources
                .food,
            91_234.5,
            "trusted cache remains authoritative"
        );

        let mut tick_broadcast = state.snapshots.subscribe();
        let tick_private_id = private_id.to_owned();
        run_tick_once(state.clone(), 1, 1_001_000, move |world, _| {
            let private = world
                .colonies
                .iter_mut()
                .find(|colony| colony.id == tick_private_id)
                .expect("tick private village");
            private.resources.water = 73_456.25;
            private
                .stockpiles
                .iter_mut()
                .find(|pile| pile.is_general_storehouse())
                .expect("tick storehouse")
                .contents
                .water = 73_456.25;
        })
        .await
        .expect("tick worker");
        let canonical_tick = tick_broadcast.recv().await.expect("tick broadcast");
        assert_eq!(canonical_tick.colonies[1].resources.water, 73_456.25);
        let directory = state.village_directory.read().await;
        let projected_tick = project_snapshot(
            canonical_tick,
            &directory,
            connection.identity.as_ref(),
            &connection.colony_id,
        );
        drop(directory);
        assert_safe_wire(&projected_tick, "broadcast tick");

        let result = send_action(
            &state,
            &mut connection,
            &ClientAction::SetCatLaborPreference {
                session_id: signed.session_id.clone(),
                nickname: "Bookkeeper".to_owned(),
                sig: signed.sig.clone(),
                cat_id,
                labor: cat_protocol::Labor::Haul,
                enabled: false,
            },
        )
        .await;
        assert!(result.result.ok, "signed action: {result:?}");

        let mut reconnected =
            ConnectionContext::new("accountant-reconnect".to_owned(), private_id.to_owned());
        reconnected.identity = Some(signed);
        let after_action = current_snapshot(&state, 1, &reconnected).await;
        assert_safe_wire(&after_action, "post-action reconnect");
        assert_eq!(
            state.completed_snapshot.read().await.colonies[1]
                .resources
                .water,
            73_456.25,
            "post-action trusted cache remains authoritative"
        );
    }

    #[test]
    fn websocket_snapshot_serialization_handles_a_populated_zero_den_legacy_colony() {
        let mut world = new_world(WORLD_SEED);
        let mut colony = found_colony(
            WORLD_SEED,
            STARTER_COLONY_ID,
            1_000_000,
            STARTER_COLONY_SEED,
        );
        colony
            .buildings
            .retain(|building| building.building_type != cat_sim::types::BuildingType::Den);
        world.colonies.push(colony);

        let snapshot = build_snapshot(&world, 1_000_000, 1);
        assert_eq!(snapshot.colonies[0].housing.capacity, 0);
        assert!(snapshot.colonies[0].housing.pressure.is_finite());
        let websocket_text =
            serde_json::to_string(&snapshot).expect("send_snapshot serialization path");
        let decoded: WorldSnapshot =
            serde_json::from_str(&websocket_text).expect("client websocket decode");
        assert_eq!(decoded, snapshot);
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
        let mut connection =
            ConnectionContext::new("test-connection".to_owned(), STARTER_COLONY_ID.to_owned());
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

    #[tokio::test]
    async fn signed_building_plans_report_the_catalogs_founding_and_milling_truth() {
        let hut_state = build_state(1_000_000);
        let (mut hut_connection, hut_session) = authenticated_connection(&hut_state);
        let hut = send_action(
            &hut_state,
            &mut hut_connection,
            &ClientAction::PlanBuilding {
                session_id: hut_session.session_id.clone(),
                nickname: "Builder".to_owned(),
                sig: hut_session.sig.clone(),
                building_type: cat_protocol::BuildingType::ResearchHut,
                site: None,
            },
        )
        .await;
        assert!(hut.result.ok, "signed founding hut denied: {hut:?}");

        let mill_state = build_state(1_000_000);
        mill_state.world.lock().await.colonies[0]
            .upgrade_tree
            .owned_node_ids
            .push("mill_foundations".to_owned());
        let (mut mill_connection, mill_session) = authenticated_connection(&mill_state);
        let mill_action = ClientAction::PlanBuilding {
            session_id: mill_session.session_id.clone(),
            nickname: "Builder".to_owned(),
            sig: mill_session.sig.clone(),
            building_type: cat_protocol::BuildingType::Mill,
            site: None,
        };
        let denied = send_action(&mill_state, &mut mill_connection, &mill_action).await;
        assert!(!denied.result.ok);
        assert_eq!(
            denied.result.message.as_deref(),
            Some("Research Milling before construction.")
        );

        mill_state.world.lock().await.colonies[0]
            .upgrade_tree
            .owned_node_ids
            .push("milling".to_owned());
        let accepted = send_action(&mill_state, &mut mill_connection, &mill_action).await;
        assert!(
            accepted.result.ok,
            "Milling did not unlock Mill: {accepted:?}"
        );
    }

    #[tokio::test]
    async fn authenticated_personal_and_communal_founders_can_plan_all_three_benches() {
        let bench_types = [
            cat_protocol::BuildingType::WoodCutter,
            cat_protocol::BuildingType::StonePrep,
            cat_protocol::BuildingType::Woodworking,
        ];

        let communal_state = build_state(1_000_000);
        let (mut communal_connection, communal_session) = authenticated_connection(&communal_state);
        assert!(
            !communal_state.world.lock().await.colonies[0]
                .upgrade_tree
                .owned_node_ids
                .iter()
                .any(|node| node == "basic_tools")
        );
        for building_type in bench_types {
            let placed = send_action(
                &communal_state,
                &mut communal_connection,
                &ClientAction::PlanBuilding {
                    session_id: communal_session.session_id.clone(),
                    nickname: "Communal Builder".to_owned(),
                    sig: communal_session.sig.clone(),
                    building_type,
                    site: None,
                },
            )
            .await;
            assert!(
                placed.result.ok,
                "communal {building_type:?} placement denied: {placed:?}"
            );
        }

        let secret = "personal-bench-secret";
        let owner = signed_session("personal-builder".to_owned(), secret);
        let mut world = starter_world(1_000_000);
        let mut personal =
            cat_sim::world_tick::found_colony(WORLD_SEED, "personal-benches", 1_000_000, 991);
        personal.kind = VillageKind::Personal;
        personal.owner_player_id = Some(owner.player_id.clone());
        world.colonies.push(personal);
        let conn = Connection::open_in_memory().expect("personal bench database");
        persistence::init_schema(&conn).expect("personal bench schema");
        let personal_state =
            build_state_from_world(world, conn, secret.to_owned(), false, 1_000_000);
        let mut personal_connection = ConnectionContext::new(
            "personal-builder-socket".to_owned(),
            "personal-benches".to_owned(),
        );
        personal_connection.identity = Some(owner.clone());
        for building_type in bench_types {
            let placed = send_action(
                &personal_state,
                &mut personal_connection,
                &ClientAction::PlanBuilding {
                    session_id: owner.session_id.clone(),
                    nickname: "Personal Builder".to_owned(),
                    sig: owner.sig.clone(),
                    building_type,
                    site: None,
                },
            )
            .await;
            assert!(
                placed.result.ok,
                "personal {building_type:?} placement denied: {placed:?}"
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

    fn build_test_state_from_world(world: WorldState, now_ms: i64) -> AppState {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        persistence::init_schema(&conn).expect("init in-memory schema");
        build_state_from_world(
            world,
            conn,
            "guided-campaign-secret".to_owned(),
            false,
            now_ms,
        )
    }

    fn steward_stockpile_campaign_world(seed: u32, started_at: i64) -> WorldState {
        let mut world = new_world(seed);
        let mut colony = found_colony(seed, STARTER_COLONY_ID, started_at, seed);
        colony
            .upgrade_tree
            .owned_node_ids
            .push("basic_tools".to_owned());
        for (from, into) in [
            (
                cat_sim::types::BuildingType::Woodworking,
                cat_sim::types::BuildingType::Workshop,
            ),
            (
                cat_sim::types::BuildingType::StonePrep,
                cat_sim::types::BuildingType::Mill,
            ),
            (
                cat_sim::types::BuildingType::WoodCutter,
                cat_sim::types::BuildingType::Sawmill,
            ),
            (
                cat_sim::types::BuildingType::Den,
                cat_sim::types::BuildingType::Smelter,
            ),
        ] {
            let building = colony
                .buildings
                .iter_mut()
                .find(|building| building.building_type == from)
                .expect("founding fixture has a convertible station");
            building.building_type = into;
            building.is_complete = true;
            building.construction_progress = 100;
            building.assigned_cat = None;
            building.production_queue = cat_sim::world_tick::default_production_queue(into);
        }
        colony.resources.grain = 60.0;
        colony.resources.flour = 40.0;
        colony.resources.materials = 60.0;
        colony.resources.refined = 40.0;
        colony.resources.logs = 60.0;
        colony.resources.lumber = 40.0;
        colony.resources.ore = 60.0;
        colony.resources.metal = 40.0;
        cat_sim::world_tick::reconcile_colony_stockpiles(&mut colony);
        world.colonies.push(colony);
        world
    }

    #[tokio::test]
    async fn signed_steward_campaign_creates_persisted_physical_piles_while_passive_twin_does_not()
    {
        let started_at = 1_000_000;
        let initial = steward_stockpile_campaign_world(77, started_at);
        let guided = build_test_state_from_world(initial.clone(), started_at);
        let passive = build_test_state_from_world(initial, started_at);
        let mut connection =
            ConnectionContext::new("steward-player".to_owned(), STARTER_COLONY_ID.to_owned());
        let presence = send_action(
            &guided,
            &mut connection,
            &ClientAction::Presence {
                session_id: String::new(),
                nickname: "Logistics Cat".to_owned(),
                sig: None,
            },
        )
        .await;
        assert!(
            presence.result.ok,
            "presence handshake failed: {presence:?}"
        );
        let signed = connection
            .identity
            .as_ref()
            .expect("presence binds signed identity")
            .clone();
        let steward_id = guided.world.lock().await.colonies[0].cats[0].id.clone();
        let appointed = send_action(
            &guided,
            &mut connection,
            &ClientAction::AssignOfficer {
                session_id: signed.session_id,
                nickname: "Logistics Cat".to_owned(),
                sig: signed.sig,
                role: OfficerRole::Steward,
                cat_id: steward_id,
            },
        )
        .await;
        assert!(
            appointed.result.ok,
            "signed appointment failed: {appointed:?}"
        );

        {
            let mut world = guided.world.lock().await;
            let _ = world_tick(&mut world, started_at + 1_000);
            let colony = &world.colonies[0];
            assert_eq!(colony.stock_ledger.steward_managed_piles.len(), 9);
            assert!(colony.jobs.iter().any(|job| matches!(
                job.metadata,
                cat_sim::world_tick::JobMetadata::StockpileHaul { .. }
            )));
            let conn = guided.db.lock().await;
            persistence::save_world(&conn, &world).expect("save guided Steward world");
        }
        {
            let mut world = passive.world.lock().await;
            let _ = world_tick(&mut world, started_at + 1_000);
            assert!(
                world.colonies[0]
                    .stock_ledger
                    .steward_managed_piles
                    .is_empty()
            );
            assert!(!world.colonies[0].jobs.iter().any(|job| matches!(
                job.metadata,
                cat_sim::world_tick::JobMetadata::StockpileHaul { .. }
            )));
        }
        let restarted = {
            let conn = guided.db.lock().await;
            persistence::load_world(&conn)
                .expect("load guided Steward world")
                .expect("saved world exists")
        };
        let current = guided.world.lock().await.clone();
        assert_eq!(
            restarted.colonies[0].stock_ledger, current.colonies[0].stock_ledger,
            "restart preserves managed provenance"
        );
        assert_eq!(
            restarted.colonies[0].stockpiles,
            current.colonies[0].stockpiles
        );
        assert_eq!(restarted.colonies[0].jobs, current.colonies[0].jobs);
        assert_eq!(restarted.colonies[0].officers, current.colonies[0].officers);
    }

    fn establish_campaign_core(colony: &mut cat_sim::world_tick::ColonyRuntime) {
        colony.resources.materials = 1_000.0;
        colony.resources.lumber = 100.0;
        colony.resources.blocks = 100.0;
        colony.resources.food = 300.0;
        colony.resources.water = 300.0;
        for (index, (role, from, building_type, upgrade)) in [
            (
                cat_sim::officers::OfficerRole::Steward,
                cat_sim::types::BuildingType::Woodworking,
                cat_sim::types::BuildingType::Workshop,
                "basic_tools",
            ),
            (
                cat_sim::officers::OfficerRole::Forester,
                cat_sim::types::BuildingType::WoodCutter,
                cat_sim::types::BuildingType::Sawmill,
                "sawmill",
            ),
            (
                cat_sim::officers::OfficerRole::Captain,
                cat_sim::types::BuildingType::StonePrep,
                cat_sim::types::BuildingType::Barracks,
                "barracks",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            if !colony
                .upgrade_tree
                .owned_node_ids
                .iter()
                .any(|owned| owned == upgrade)
            {
                colony.upgrade_tree.owned_node_ids.push(upgrade.to_owned());
            }
            let building = colony
                .buildings
                .iter_mut()
                .find(|building| building.building_type == from)
                .expect("founding blueprint contains the compatible office yard");
            building.building_type = building_type;
            building.production_queue =
                cat_sim::world_tick::default_production_queue(building_type);
            colony.officers.insert(role, colony.cats[index].id.clone());
        }
    }

    fn prepare_authenticated_den_branch(
        colony: &mut cat_sim::world_tick::ColonyRuntime,
        world_seed: u32,
    ) -> cat_sim::world_tick::TilePos {
        let workshop = colony
            .buildings
            .iter()
            .find(|building| building.building_type == cat_sim::types::BuildingType::Workshop)
            .expect("campaign setup has a Steward workshop");
        let den_site = workshop.position;
        let mut released = workshop
            .assigned_cat
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        released.extend(
            colony
                .jobs
                .iter()
                .filter(|job| job.kind == cat_sim::types::JobKind::LeaderPlanHouse)
                .filter_map(|job| job.assigned_cat.clone()),
        );
        colony
            .officers
            .remove(&cat_sim::officers::OfficerRole::Steward);
        colony
            .buildings
            .retain(|building| building.building_type != cat_sim::types::BuildingType::Workshop);
        colony
            .jobs
            .retain(|job| job.kind != cat_sim::types::JobKind::LeaderPlanHouse);
        for cat in &mut colony.cats {
            if released.contains(&cat.id) {
                cat.current_task = None;
                cat.activity = cat_sim::entities::CatActivity::Idle;
                cat.destination = None;
            }
        }
        assert!(cat_sim::world_tick::can_plan_building_at(
            colony,
            den_site,
            world_seed,
            cat_sim::types::BuildingType::Den,
        ));
        den_site
    }

    #[tokio::test]
    async fn signed_player_den_retains_migrant_while_unattended_twin_loses_them() {
        const SEED: u32 = 7;
        const TICK_MS: i64 = 15 * 60_000;
        const HORIZON_HOURS: i64 = 120;

        let started_at = now_ms();
        let mut initial_world = new_world(SEED);
        let mut colony = found_colony(SEED, STARTER_COLONY_ID, started_at, SEED);
        // The test isolates authenticated housing, so give both unattended twins
        // the legal established core that owns their survival/prosperity loop.
        establish_campaign_core(&mut colony);
        // Keep this comparison about the signed player's extra Den. A staffed
        // Steward legitimately starts its own housing scaffold before the first
        // migrant's probation is observed; now that scaffold inputs are physical,
        // it can still be hauling when the signed request arrives. Leaving the
        // already-researched Workshop in place but vacating the office prevents a
        // competing autonomous project in both otherwise-identical branches.
        colony
            .officers
            .remove(&cat_sim::officers::OfficerRole::Steward);
        initial_world.colonies.push(colony);
        let guided = build_test_state_from_world(initial_world.clone(), started_at);
        let unattended = build_test_state_from_world(initial_world, started_at);

        // Exercise the real production handshake: the server issues the signed
        // session, binds it to this connection, and the following build action is
        // authenticated through the same HMAC path as a websocket client.
        let mut connection =
            ConnectionContext::new("guided-player".to_owned(), STARTER_COLONY_ID.to_owned());
        let presence = send_action(
            &guided,
            &mut connection,
            &ClientAction::Presence {
                session_id: String::new(),
                nickname: "Builder Cat".to_owned(),
                sig: None,
            },
        )
        .await;
        assert!(
            presence.result.ok,
            "presence handshake failed: {presence:?}"
        );
        let session_id = presence.fields["sessionId"].clone();
        let sig = presence.fields["sig"].clone();
        assert!(verify_session(
            session_id.as_str(),
            Some(sig.as_str()),
            guided.session_secret.as_str()
        ));

        let mut plan_sent = false;
        let mut guided_arrival_id = None;
        let mut unattended_arrival_id = None;
        let mut finished_at = started_at;
        for step in 1..=HORIZON_HOURS * 60 / 15 {
            let tick_now = started_at + step * TICK_MS;
            finished_at = tick_now;
            for state in [&guided, &unattended] {
                let mut world = state.world.lock().await;
                let reports = world_tick(&mut world, tick_now);
                assert_eq!(reports[0].reset_reason, None, "tick {step} reset a twin");
            }

            if guided_arrival_id.is_none() {
                guided_arrival_id = guided.world.lock().await.colonies[0]
                    .migration_state
                    .probationary_migrants
                    .first()
                    .map(|migrant| migrant.id.clone());
                unattended_arrival_id = unattended.world.lock().await.colonies[0]
                    .migration_state
                    .probationary_migrants
                    .first()
                    .map(|migrant| migrant.id.clone());
            }

            if guided_arrival_id.is_some() && !plan_sent {
                assert_eq!(guided_arrival_id, unattended_arrival_id);
                let guided_site = {
                    let mut world = guided.world.lock().await;
                    prepare_authenticated_den_branch(&mut world.colonies[0], SEED)
                };
                let unattended_site = {
                    let mut world = unattended.world.lock().await;
                    prepare_authenticated_den_branch(&mut world.colonies[0], SEED)
                };
                assert_eq!(guided_site, unattended_site);
                let result = send_action(
                    &guided,
                    &mut connection,
                    &ClientAction::PlanBuilding {
                        session_id: session_id.clone(),
                        nickname: "Builder Cat".to_owned(),
                        sig: sig.clone(),
                        building_type: cat_protocol::BuildingType::Den,
                        site: Some(cat_protocol::TilePoint {
                            x: guided_site.x,
                            y: guided_site.y,
                        }),
                    },
                )
                .await;
                assert!(result.result.ok, "signed den plan failed: {result:?}");
                plan_sent = true;
            }

            if plan_sent {
                let guided_world = guided.world.lock().await;
                let unattended_world = unattended.world.lock().await;
                let guided_colony = &guided_world.colonies[0];
                let unattended_colony = &unattended_world.colonies[0];
                let guided_retained = guided_arrival_id.as_ref().is_some_and(|id| {
                    guided_colony
                        .cats
                        .iter()
                        .any(|cat| cat.id == *id && cat.death_time.is_none())
                        && guided_colony
                            .migration_state
                            .probationary_migrants
                            .is_empty()
                });
                let unattended_departed = unattended_arrival_id.as_ref().is_some_and(|id| {
                    !unattended_colony.cats.iter().any(|cat| cat.id == *id)
                        && unattended_colony.migration_departures > 0
                });
                if guided_retained && unattended_departed {
                    break;
                }
            }
        }

        assert!(
            plan_sent,
            "organic campaign never reached a migrant arrival"
        );
        let guided_arrival_id = guided_arrival_id.expect("guided migrant id");
        let unattended_arrival_id = unattended_arrival_id.expect("unattended migrant id");
        {
            let world = guided.world.lock().await;
            let colony = &world.colonies[0];
            assert!(
                colony
                    .cats
                    .iter()
                    .any(|cat| cat.id == guided_arrival_id && cat.death_time.is_none())
            );
            assert!(
                !colony
                    .migration_state
                    .probationary_migrants
                    .iter()
                    .any(|migrant| migrant.id == guided_arrival_id)
            );
        }
        {
            let world = unattended.world.lock().await;
            assert!(
                !world.colonies[0]
                    .cats
                    .iter()
                    .any(|cat| cat.id == unattended_arrival_id)
            );
        }
        let guided_snapshot = {
            let world = guided.world.lock().await;
            build_snapshot(&world, finished_at, 1)
        };
        let unattended_snapshot = {
            let world = unattended.world.lock().await;
            build_snapshot(&world, finished_at, 1)
        };
        let guided_colony = &guided_snapshot.colonies[0];
        let unattended_colony = &unattended_snapshot.colonies[0];
        assert!(guided_colony.housing.capacity >= 20);
        // A later organic cohort may already be waiting by the time the unattended
        // twin records its departure. The contract under test is that the original
        // signed-action cohort became permanent, asserted by id above; do not mistake
        // healthy continued migration for a failure to retain that cohort.
        assert!(guided_colony.housing.population >= 16);
        assert_eq!(unattended_colony.housing.capacity, 15);
        assert_eq!(unattended_colony.housing.housed, 15);
        assert!(unattended_colony.housing.departures > 0);
    }

    #[tokio::test]
    async fn found_village_action_updates_shared_snapshot() {
        let state = build_state(1_000_000);
        let action = ClientAction::FoundVillage {
            name: "Newford".to_owned(),
            session_id: "session-1".to_owned(),
            sig: None,
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
            sig: None,
        };

        let encoded = serde_json::to_string(&action).expect("serialize action");
        let decoded: ClientAction = serde_json::from_str(&encoded).expect("deserialize action");

        assert_eq!(decoded, action);
    }

    #[tokio::test]
    async fn actions_require_a_socket_bound_presence_identity() {
        let state = build_state(1_000_000);
        let mut connection =
            ConnectionContext::new("connection-a".to_owned(), STARTER_COLONY_ID.to_owned());
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
            sig: None,
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
        let mut actions = Vec::new();
        for role in [
            OfficerRole::Steward,
            OfficerRole::Accountant,
            OfficerRole::Forester,
            OfficerRole::Farmer,
            OfficerRole::Captain,
            OfficerRole::Loremaster,
            OfficerRole::ClothLeader,
        ] {
            actions.push(ClientAction::AssignOfficer {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                role,
                cat_id: cat_id.clone(),
            });
            actions.push(ClientAction::UnassignOfficer {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                role,
            });
        }
        actions.extend([
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
            ClientAction::DesignateFishingSpot {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                at: TilePoint { x: 8, y: 8 },
            },
            ClientAction::RemoveGatherSpot {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                stockpile_id: "gather-1".to_owned(),
            },
            ClientAction::ResearchNode {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                node_id: "research_hut".to_owned(),
            },
            ClientAction::OfferTithe {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
            },
            ClientAction::OfferMaterials {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
            },
            ClientAction::HaulGatherSpot {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                stockpile_id: "gather-1".to_owned(),
                cat_id: None,
            },
            ClientAction::SetCatLaborPreference {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                cat_id: cat_id.clone(),
                labor: cat_protocol::Labor::Process,
                enabled: true,
            },
            ClientAction::EditProductionQueue {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                building_id: "sawmill-1".to_owned(),
                edit: cat_protocol::ProductionQueueEdit::SetPaused { paused: true },
            },
            ClientAction::DispatchScout {
                session_id: signed.session_id,
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                mission: ScoutMission::Explore,
            },
        ]);

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
    async fn authenticated_player_can_designate_a_shore_and_order_physical_fishing() {
        let state = build_state(1_000_000);
        let (mut connection, signed) = authenticated_connection(&state);
        let bank = {
            let mut world = state.world.lock().await;
            let seed = world.world_seed;
            let colony = &mut world.colonies[0];
            let bank = colony
                .world_tiles
                .keys()
                .copied()
                .find(|site| {
                    colony.revealed_tiles.contains(site)
                        && cat_sim::world_tick::stockpile_placement_error(
                            colony,
                            cat_sim::zones::ZoneRect {
                                x1: site.x,
                                y1: site.y,
                                x2: site.x,
                                y2: site.y,
                            },
                            seed,
                            false,
                        )
                        .is_none()
                        && colony
                            .world_tiles
                            .contains_key(&cat_sim::world_tick::TilePos {
                                x: site.x,
                                y: site.y - 1,
                            })
                        && {
                            let water = cat_sim::world_tick::TilePos {
                                x: site.x,
                                y: site.y - 1,
                            };
                            let mut projected = colony.clone();
                            projected.revealed_tiles.insert(water);
                            let tile = projected.world_tiles.get_mut(&water).unwrap();
                            tile.tile_type = cat_sim::types::TileType::River;
                            tile.resources.water = 100;
                            cat_sim::world_tick::is_reachable_fishing_shore(&projected, *site, seed)
                        }
                })
                .expect("founding map has a clear bank fixture");
            let water = cat_sim::world_tick::TilePos {
                x: bank.x,
                y: bank.y - 1,
            };
            colony.revealed_tiles.insert(water);
            let tile = colony.world_tiles.get_mut(&water).unwrap();
            tile.tile_type = cat_sim::types::TileType::River;
            tile.resources.water = 100;
            bank
        };
        let designation = ClientAction::DesignateFishingSpot {
            session_id: signed.session_id.clone(),
            nickname: "Angler".to_owned(),
            sig: signed.sig.clone(),
            at: TilePoint {
                x: bank.x,
                y: bank.y,
            },
        };
        let result = send_action(&state, &mut connection, &designation).await;
        assert!(result.result.ok, "{result:?}");

        let request = ClientAction::RequestJob {
            session_id: signed.session_id,
            nickname: "Angler".to_owned(),
            sig: signed.sig,
            kind: cat_protocol::JobKind::Fish,
        };
        let result = send_action(&state, &mut connection, &request).await;
        assert!(result.result.ok, "{result:?}");

        let world = state.world.lock().await;
        assert!(
            world.colonies[0]
                .gather_spots
                .iter()
                .any(|spot| { spot.purpose == cat_sim::stockpiles::GatherSpotPurpose::Fishing })
        );
        assert!(world.colonies[0].jobs.iter().any(|job| {
            job.kind == cat_sim::types::JobKind::Fish
                && job.requested_by == cat_sim::world_tick::JobRequester::Player
        }));
    }

    #[tokio::test]
    async fn signed_logging_uses_the_catalog_sawmill_entitlement() {
        let state = build_state(1_000_000);
        let (mut connection, signed) = authenticated_connection(&state);
        let request = ClientAction::RequestJob {
            session_id: signed.session_id,
            nickname: "Logger".to_owned(),
            sig: signed.sig,
            kind: cat_protocol::JobKind::GatherLogs,
        };

        let before_locked = state.world.lock().await.clone();
        let locked = send_action(&state, &mut connection, &request).await;
        assert!(!locked.result.ok);
        assert_eq!(
            locked.result.message.as_deref(),
            Some("Research Sawmill before requesting logging.")
        );
        assert_eq!(
            *state.world.lock().await,
            before_locked,
            "a signed denied request must not mutate the world"
        );

        {
            let mut world = state.world.lock().await;
            let seed = world.world_seed;
            let tree = (-12..=12)
                .flat_map(|chunk_y| (-12..=12).map(move |chunk_x| (chunk_x, chunk_y)))
                .find_map(|(chunk_x, chunk_y)| {
                    cat_sim::terrain_gen::generate_terrain_chunk(
                        chunk_x,
                        chunk_y,
                        i64::from(seed),
                        cat_sim::terrain_gen::WORLD_TERRAIN_OPTIONS,
                    )
                    .into_iter()
                    .find(|tile| {
                        matches!(
                            tile.decoration,
                            Some(cat_sim::terrain_gen::DecorationRole::Tree { .. })
                        ) && tile.climate_biome.properties().resource
                            == cat_sim::climate::ResourceHint::Wood
                    })
                    .map(|tile| cat_sim::world_tick::TilePos {
                        x: tile.x,
                        y: tile.y,
                    })
                })
                .expect("bounded climate scan contains a logging tree");
            let colony = &mut world.colonies[0];
            colony
                .upgrade_tree
                .owned_node_ids
                .push("sawmill".to_owned());
            let mut logging_tile = colony
                .world_tiles
                .values()
                .next()
                .expect("founding world tile")
                .clone();
            logging_tile.pos = tree;
            logging_tile.path_wear = 63;
            logging_tile.overlay_feature = None;
            colony.world_tiles.insert(tree, logging_tile);
        }

        let accepted = send_action(&state, &mut connection, &request).await;
        assert!(accepted.result.ok, "{accepted:?}");
        assert!(state.world.lock().await.colonies[0].jobs.iter().any(|job| {
            job.kind == cat_sim::types::JobKind::GatherLogs
                && job.requested_by == cat_sim::world_tick::JobRequester::Player
        }));
    }

    #[tokio::test]
    async fn signed_labor_and_station_guidance_survives_database_restart() {
        let path = std::env::temp_dir().join(format!(
            "cat-server-guidance-restart-{}-{}.db",
            std::process::id(),
            NEXT_DATABASE_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_file(&path);
        let secret = "guidance-restart-secret";
        let mut world = starter_world(1_000_000);
        for (id, building_type, x) in [
            (
                "guidance-sawmill",
                cat_sim::types::BuildingType::Sawmill,
                40,
            ),
            ("guidance-mill", cat_sim::types::BuildingType::Mill, 44),
            (
                "guidance-workshop",
                cat_sim::types::BuildingType::Workshop,
                48,
            ),
            (
                "guidance-smelter",
                cat_sim::types::BuildingType::Smelter,
                52,
            ),
        ] {
            world.colonies[0]
                .buildings
                .push(cat_sim::world_tick::BuildingRuntime {
                    id: id.to_owned(),
                    building_type,
                    position: cat_sim::world_tick::TilePos { x, y: 40 },
                    is_complete: true,
                    construction_progress: 100,
                    production_queue: Vec::new(),
                    ..cat_sim::world_tick::BuildingRuntime::default()
                });
        }
        world.colonies[0].upgrade_tree.research_points = 1_000.0;
        let cat_id = world.colonies[0].cats[0].id.clone();
        let station_recipes = [
            ("guidance-sawmill", cat_sim::world_tick::SAWMILL_RECIPE_ID),
            ("guidance-mill", cat_sim::world_tick::MILL_RECIPE_ID),
            ("guidance-workshop", cat_sim::world_tick::WORKSHOP_RECIPE_ID),
            ("guidance-smelter", cat_sim::world_tick::SMELTER_RECIPE_ID),
        ];
        let conn = Connection::open(&path).expect("open guidance database");
        persistence::init_schema(&conn).expect("init guidance database");
        let state = build_state_from_world(world, conn, secret.to_owned(), false, 1_000_000);
        let signed = signed_session("guidance-session".to_owned(), secret);
        let mut connection =
            ConnectionContext::new("guidance-socket".to_owned(), STARTER_COLONY_ID.to_owned());
        connection.identity = Some(signed.clone());

        for node_id in [
            "research_hut",
            "basic_tools",
            "water_carriers",
            "den_insulation",
            "foraging_lore",
            "sawmill",
            "masonry",
            "irrigation",
            "mountaineering",
            "smelting",
            "school",
            "advanced_storage",
            "carpentry_sources",
            "carpentry_preparation",
            "grain_milling_sources",
            "grain_milling_preparation",
            "metallurgy_sources",
            "metallurgy_preparation",
            "trade_goods_sources",
            "trade_goods_preparation",
        ] {
            let result = send_action(
                &state,
                &mut connection,
                &ClientAction::ResearchNode {
                    session_id: signed.session_id.clone(),
                    nickname: "Guide".to_owned(),
                    sig: signed.sig.clone(),
                    node_id: node_id.to_owned(),
                },
            )
            .await;
            assert!(
                result.result.ok,
                "signed research {node_id} failed: {result:?}"
            );
        }
        assert_eq!(
            state.world.lock().await.colonies[0].last_leader_research_choice_at,
            None,
            "many signed player purchases in one real day must not consume the Leader clock"
        );

        let preference = ClientAction::SetCatLaborPreference {
            session_id: signed.session_id.clone(),
            nickname: "Guide".to_owned(),
            sig: signed.sig.clone(),
            cat_id: cat_id.clone(),
            labor: cat_protocol::Labor::Process,
            enabled: true,
        };
        let result = send_action(&state, &mut connection, &preference).await;
        assert!(result.result.ok, "signed preference failed: {result:?}");
        for (index, (building_id, recipe_id)) in station_recipes.iter().enumerate() {
            let action = ClientAction::EditProductionQueue {
                session_id: signed.session_id.clone(),
                nickname: "Guide".to_owned(),
                sig: signed.sig.clone(),
                building_id: (*building_id).to_owned(),
                edit: cat_protocol::ProductionQueueEdit::Add {
                    recipe_id: (*recipe_id).to_owned(),
                    repeat: true,
                },
            };
            let result = send_action(&state, &mut connection, &action).await;
            assert!(result.result.ok, "signed queue add failed: {result:?}");
            if index < 2 {
                for edit in [
                    cat_protocol::ProductionQueueEdit::SetRepeat {
                        index: 0,
                        repeat: false,
                    },
                    cat_protocol::ProductionQueueEdit::SetPaused { paused: true },
                ] {
                    let result = send_action(
                        &state,
                        &mut connection,
                        &ClientAction::EditProductionQueue {
                            session_id: signed.session_id.clone(),
                            nickname: "Guide".to_owned(),
                            sig: signed.sig.clone(),
                            building_id: (*building_id).to_owned(),
                            edit,
                        },
                    )
                    .await;
                    assert!(result.result.ok, "signed queue edit failed: {result:?}");
                }
            }
        }
        save_current_world(&state)
            .await
            .expect("persist signed guidance");
        drop(state);

        let conn = Connection::open(&path).expect("reopen guidance database");
        persistence::init_schema(&conn).expect("migrate guidance database");
        let restarted = build_state_from_connection(2_000_000, conn, secret.to_owned())
            .expect("restore signed guidance");
        let restored = restarted.world.lock().await;
        let colony = &restored.colonies[0];
        assert!(
            colony
                .cats
                .iter()
                .find(|cat| cat.id == cat_id)
                .expect("guided cat restored")
                .preferred_labors
                .contains(&cat_sim::skills::Labor::Process)
        );
        for (index, (building_id, recipe_id)) in station_recipes.iter().enumerate() {
            let building = colony
                .buildings
                .iter()
                .find(|building| building.id == *building_id)
                .expect("guided station restored");
            assert_eq!(building.production_queue.len(), 1);
            assert_eq!(building.production_queue[0].recipe_id, *recipe_id);
            assert_eq!(building.production_queue[0].repeat, index >= 2);
            assert_eq!(building.production_paused, index < 2);
        }
        for study in [
            "carpentry_preparation",
            "grain_milling_preparation",
            "metallurgy_preparation",
            "trade_goods_preparation",
        ] {
            assert!(
                colony
                    .upgrade_tree
                    .owned_node_ids
                    .contains(&study.to_owned())
            );
        }
        assert_eq!(
            colony.recipe_entitlement_rules_version,
            cat_sim::world_tick::CURRENT_RECIPE_ENTITLEMENT_RULES_VERSION
        );
        assert_eq!(
            colony.last_leader_research_choice_at, None,
            "restart must preserve the unlimited manual-purchase/Leader-clock split"
        );
        drop(restored);
        drop(restarted);
        fs::remove_file(path).expect("remove guidance database");
    }

    #[tokio::test]
    async fn signed_trader_buy_sell_depletes_exact_finite_stock_across_restart() {
        let path = std::env::temp_dir().join(format!(
            "cat-server-trader-restart-{}-{}.db",
            std::process::id(),
            NEXT_DATABASE_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_file(&path);
        let secret = "trader-restart-secret";
        let signed = signed_session("trader-session".to_owned(), secret);
        let mut world = starter_world(1_000_000);
        let item = cat_sim::items::Item::new(
            cat_sim::items::ItemKind::Mug,
            cat_sim::items::Material::Wood,
            1,
        );
        world.colonies[0].add_item(item, 1);
        let item_id = world.colonies[0]
            .items
            .instances()
            .next()
            .unwrap()
            .id
            .clone();
        world.colonies[0].coin = 20.0;
        world.colonies[0].trader_visit_count = 3;
        world.colonies[0].trader = Some(cat_sim::world_tick::TraderRuntime {
            id: "signed-trader-3".to_owned(),
            position: cat_sim::entities::Position {
                map: cat_sim::entities::MapType::World,
                x: 7.0,
                y: 8.0,
            },
            destination: Some(cat_sim::entities::Position {
                map: cat_sim::entities::MapType::World,
                x: 7.0,
                y: 8.0,
            }),
            state: cat_sim::trader::TraderState::Trading,
            arrived_at: Some(1_000_000),
            depart_at: Some(i64::MAX),
            route_exterior: Some([7, 20]),
            visit_destination: Some([7, 8]),
            visit_number: 3,
            stock: std::collections::BTreeMap::from([(
                cat_sim::stockpiles::ResourceKind::Food,
                2.0,
            )]),
            items: cat_sim::items::ItemStore::default(),
            coin: 50.0,
        });
        let conn = Connection::open(&path).expect("open trader database");
        persistence::init_schema(&conn).expect("init trader database");
        let state = build_state_from_world(world, conn, secret.to_owned(), false, 1_000_000);
        let mut connection =
            ConnectionContext::new("trader-socket".to_owned(), STARTER_COLONY_ID.to_owned());
        connection.identity = Some(signed.clone());

        let sold = send_action(
            &state,
            &mut connection,
            &ClientAction::SellGoods {
                session_id: signed.session_id.clone(),
                nickname: "Trader Guide".to_owned(),
                sig: signed.sig.clone(),
                kind: "mug".to_owned(),
                material: "wood".to_owned(),
                quality: 1,
                count: 1,
            },
        )
        .await;
        assert!(sold.result.ok, "signed sale failed: {sold:?}");
        let bought = send_action(
            &state,
            &mut connection,
            &ClientAction::BuyResource {
                session_id: signed.session_id.clone(),
                nickname: "Trader Guide".to_owned(),
                sig: signed.sig.clone(),
                resource: ResourceKind::Food,
                amount: 1.0,
            },
        )
        .await;
        assert!(bought.result.ok, "signed purchase failed: {bought:?}");
        save_current_world(&state).await.expect("persist mid-visit");
        drop(state);

        let conn = Connection::open(&path).expect("reopen trader database");
        persistence::init_schema(&conn).expect("migrate trader database");
        let restarted = build_state_from_connection(2_000_000, conn, secret.to_owned())
            .expect("restore trader visit");
        {
            let restored = restarted.world.lock().await;
            let trader = restored.colonies[0].trader.as_ref().expect("same visit");
            assert_eq!(trader.visit_number, 3);
            assert_eq!(trader.stock[&cat_sim::stockpiles::ResourceKind::Food], 1.0);
            assert_eq!(
                trader.items.instances().next().unwrap().id,
                item_id,
                "restart preserves the exact sold item identity"
            );
        }
        let mut restarted_connection =
            ConnectionContext::new("trader-socket-2".to_owned(), STARTER_COLONY_ID.to_owned());
        restarted_connection.identity = Some(signed.clone());
        let final_buy = ClientAction::BuyResource {
            session_id: signed.session_id.clone(),
            nickname: "Trader Guide".to_owned(),
            sig: signed.sig.clone(),
            resource: ResourceKind::Food,
            amount: 1.0,
        };
        let depleted = send_action(&restarted, &mut restarted_connection, &final_buy).await;
        assert!(
            depleted.result.ok,
            "final finite purchase failed: {depleted:?}"
        );
        let denied = send_action(&restarted, &mut restarted_connection, &final_buy).await;
        assert!(
            !denied.result.ok,
            "sold-out manifest accepted another purchase"
        );
        let snapshot = {
            let world = restarted.world.lock().await;
            build_snapshot(&world, 2_000_000, 1)
        };
        let food = snapshot.colonies[0]
            .trader
            .as_ref()
            .unwrap()
            .sell_offers
            .iter()
            .find(|offer| offer.resource == ResourceKind::Food)
            .unwrap();
        assert_eq!(food.available, 0.0);
        assert!(food.sold_out);
        drop(restarted);
        fs::remove_file(path).expect("remove trader database");
    }

    #[tokio::test]
    async fn authenticated_manual_accountant_resumes_the_same_physical_round_after_restart() {
        let path = std::env::temp_dir().join(format!(
            "cat-server-accountant-restart-{}-{}.db",
            std::process::id(),
            NEXT_DATABASE_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_file(&path);
        let secret = "accountant-restart-secret";
        let mut world = starter_world(1_000_000);
        let tent_id = "signed-accounting-tent".to_owned();
        let cat_id = world.colonies[0].cats[0].id.clone();
        let anchor = world.colonies[0].anchor;
        world.colonies[0]
            .buildings
            .push(cat_sim::world_tick::BuildingRuntime {
                id: tent_id.clone(),
                building_type: cat_sim::types::BuildingType::AccountingTent,
                position: anchor,
                is_complete: true,
                construction_progress: 100,
                production_queue: cat_sim::world_tick::default_production_queue(
                    cat_sim::types::BuildingType::AccountingTent,
                ),
                ..cat_sim::world_tick::BuildingRuntime::default()
            });
        let conn = Connection::open(&path).expect("open accountant database");
        persistence::init_schema(&conn).expect("init accountant database");
        let state = build_state_from_world(world, conn, secret.to_owned(), false, 1_000_000);
        let signed = signed_session("accountant-session".to_owned(), secret);
        let mut connection =
            ConnectionContext::new("accountant-socket".to_owned(), STARTER_COLONY_ID.to_owned());
        connection.identity = Some(signed.clone());
        let (truth_before, piles_before) = {
            let live = state.world.lock().await;
            (
                live.colonies[0].resources.clone(),
                live.colonies[0].stockpiles.clone(),
            )
        };

        let assigned = send_action(
            &state,
            &mut connection,
            &ClientAction::AssignWorker {
                session_id: signed.session_id.clone(),
                nickname: "Bookkeeper".to_owned(),
                sig: signed.sig.clone(),
                cat_id: cat_id.clone(),
                building_id: Some(tent_id.clone()),
            },
        )
        .await;
        assert!(assigned.result.ok, "signed assignment failed: {assigned:?}");
        {
            let live = state.world.lock().await;
            assert_eq!(live.colonies[0].resources, truth_before);
            assert_eq!(live.colonies[0].stockpiles, piles_before);
        }
        {
            let mut live = state.world.lock().await;
            let _ = cat_sim::world_tick::world_tick(&mut live, 1_001_000);
            assert!(live.colonies[0].stock_ledger.active_round.is_some());
        }
        save_current_world(&state)
            .await
            .expect("persist physical accountant");
        let before = state.world.lock().await.colonies[0].stock_ledger.clone();
        drop(state);

        let conn = Connection::open(&path).expect("reopen accountant database");
        persistence::init_schema(&conn).expect("migrate accountant database");
        let restarted = build_state_from_connection(1_002_000, conn, secret.to_owned())
            .expect("restore physical accountant");
        {
            let restored = restarted.world.lock().await;
            assert_eq!(restored.colonies[0].stock_ledger, before);
            assert_eq!(
                restored.colonies[0]
                    .buildings
                    .iter()
                    .find(|building| building.id == tent_id)
                    .and_then(|building| building.assigned_cat.as_deref()),
                Some(cat_id.as_str())
            );
        }

        // The same authenticated session can reconnect and explicitly release the worker;
        // this also proves the persisted assignment is not officer-owned ghost automation.
        let mut reconnected = ConnectionContext::new(
            "accountant-reconnected".to_owned(),
            STARTER_COLONY_ID.to_owned(),
        );
        reconnected.identity = Some(signed.clone());
        let released = send_action(
            &restarted,
            &mut reconnected,
            &ClientAction::AssignWorker {
                session_id: signed.session_id,
                nickname: "Bookkeeper".to_owned(),
                sig: signed.sig,
                cat_id,
                building_id: None,
            },
        )
        .await;
        assert!(released.result.ok, "signed release failed: {released:?}");
        {
            let mut live = restarted.world.lock().await;
            let stale_reported = live.colonies[0].stock_ledger.reported.clone();
            let stale_last_counted = live.colonies[0].stock_ledger.last_counted;
            live.colonies[0].resources.food -= 1.0;
            live.colonies[0]
                .stockpiles
                .iter_mut()
                .find(|pile| pile.is_general_storehouse())
                .expect("founding general storehouse")
                .contents
                .food -= 1.0;

            // This crosses the former 30-second unstaffed recount boundary. The signed
            // release must leave the office genuinely vacant: no authoritative stock may
            // leak into the reports without another physical pile visit.
            let _ = cat_sim::world_tick::world_tick(&mut live, 1_033_000);
            assert_eq!(live.colonies[0].stock_ledger.reported, stale_reported);
            assert_eq!(
                live.colonies[0].stock_ledger.last_counted,
                stale_last_counted
            );
            assert!(live.colonies[0].stock_ledger.active_round.is_none());
            assert!(
                !live.colonies[0]
                    .stock_ledger
                    .is_accurate(&live.colonies[0].resources)
            );
        }
        drop(restarted);
        fs::remove_file(path).expect("remove accountant database");
    }

    #[tokio::test]
    async fn authenticated_manual_officer_campaign_mutates_only_the_selected_colony() {
        use cat_sim::{
            entities::Resources,
            stockpiles::{GatherSpot, ResourceKind as SimResourceKind, Stockpile},
            types::{BuildingType as SimBuildingType, JobKind as SimJobKind},
            zones::ZoneRect,
        };

        let mut world = new_world(WORLD_SEED);
        world.colonies.push(found_colony(
            WORLD_SEED,
            STARTER_COLONY_ID,
            1_000_000,
            STARTER_COLONY_SEED,
        ));
        world
            .colonies
            .push(found_colony(WORLD_SEED, "beta", 1_000_000, 22));
        {
            let colony = &mut world.colonies[0];
            colony.resources.food = 500.0;
            colony.resources.refined = 100.0;
            colony.resources.materials = 500.0;
            colony.upgrade_tree.research_points = 100.0;
            colony
                .upgrade_tree
                .owned_node_ids
                .push("basic_tools".to_owned());
            let anchor = colony.anchor;
            let accounting = colony
                .buildings
                .iter_mut()
                .filter(|building| building.building_type == SimBuildingType::Den)
                .min_by_key(|building| {
                    ((building.position.x - anchor.x).abs(), building.position.y)
                })
                .expect("starter top-den footprint");
            accounting.id = "auth-accounting".to_owned();
            accounting.building_type = SimBuildingType::AccountingTent;
            accounting.assigned_cat = None;
            accounting.automated_by = None;
            let claimed = colony
                .claimed_tiles
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            let mut occupied = BTreeSet::new();
            for building in &colony.buildings {
                let (width, height) = cat_sim::world_tick::footprint_for(building.building_type);
                for dy in 0..height {
                    for dx in 0..width {
                        let tile = cat_sim::world_tick::TilePos {
                            x: building.position.x + dx,
                            y: building.position.y + dy,
                        };
                        assert!(claimed.contains(&tile), "fixture building is off-claim");
                        assert!(
                            occupied.insert(tile),
                            "fixture buildings overlap at {tile:?}"
                        );
                    }
                }
            }
            let gather_rect = colony
                .world_tiles
                .keys()
                .copied()
                .map(|tile| ZoneRect {
                    x1: tile.x,
                    y1: tile.y,
                    x2: tile.x,
                    y2: tile.y,
                })
                .find(|rect| {
                    cat_sim::world_tick::stockpile_placement_error(colony, *rect, WORLD_SEED, false)
                        .is_none()
                        && !colony
                            .claimed_tiles
                            .contains(&cat_sim::world_tick::TilePos {
                                x: rect.x1,
                                y: rect.y1,
                            })
                })
                .expect("campaign founding reveal has a legal exterior gather tile");
            colony.stockpiles.push(Stockpile {
                id: "auth-gather".to_owned(),
                rect: gather_rect,
                accepts: BTreeSet::from([SimResourceKind::Food]),
                contents: Resources {
                    food: 12.0,
                    ..Resources::default()
                },
            });
            colony.gather_spots.push(GatherSpot {
                stockpile_id: "auth-gather".to_owned(),
                kind: SimResourceKind::Food,
                expires_at_ms: 2_000_000,
                purpose: cat_sim::stockpiles::GatherSpotPurpose::General,
            });
        }
        {
            let id = "auth-accounting";
            let colony = &world.colonies[0];
            let building = colony
                .buildings
                .iter()
                .find(|building| building.id == id)
                .expect("fixture building remains after access pass");
            assert!(
                cat_sim::world_tick::building_is_road_connected_to_shrine(
                    colony, building, WORLD_SEED
                ),
                "{id} disconnected: officers={:?} materials={} events={:?}",
                colony.officers,
                colony.resources.materials,
                colony
                    .events
                    .iter()
                    .rev()
                    .take(5)
                    .map(|event| (&event.kind, &event.message))
                    .collect::<Vec<_>>()
            );
        }
        let beta_before = world.colonies[1].clone();
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        persistence::init_schema(&conn).expect("init in-memory schema");
        let state = build_state_from_world(
            world,
            conn,
            "test-session-secret".to_owned(),
            false,
            1_000_000,
        );
        let (mut connection, signed) = authenticated_connection(&state);
        let cat_ids = state.world.lock().await.colonies[0]
            .cats
            .iter()
            .take(1)
            .map(|cat| cat.id.clone())
            .collect::<Vec<_>>();

        let actions = [
            ClientAction::ResearchNode {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: signed.sig.clone(),
                node_id: "research_hut".to_owned(),
            },
            ClientAction::ResearchNode {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: signed.sig.clone(),
                node_id: "research_hut_foundations".to_owned(),
            },
            ClientAction::ResearchNode {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: signed.sig.clone(),
                node_id: "foraging_lore".to_owned(),
            },
            ClientAction::AssignOfficer {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: signed.sig.clone(),
                role: OfficerRole::Accountant,
                cat_id: cat_ids[0].clone(),
            },
            ClientAction::HaulGatherSpot {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: signed.sig.clone(),
                stockpile_id: "auth-gather".to_owned(),
                cat_id: None,
            },
            ClientAction::OfferMaterials {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: signed.sig.clone(),
            },
            ClientAction::OfferTithe {
                session_id: signed.session_id,
                nickname: "Tester".to_owned(),
                sig: signed.sig,
            },
        ];

        for action in actions {
            let result = send_action(&state, &mut connection, &action).await;
            assert!(
                result.result.ok,
                "authenticated {action:?} failed: {result:?}"
            );
        }

        let world = state.world.lock().await;
        let colony = &world.colonies[0];
        assert!(
            colony
                .upgrade_tree
                .owned_node_ids
                .iter()
                .any(|node| node == "research_hut")
        );
        assert!(
            colony
                .upgrade_tree
                .owned_node_ids
                .iter()
                .any(|node| node == "research_hut_foundations")
        );
        assert!(
            colony
                .upgrade_tree
                .owned_node_ids
                .iter()
                .any(|node| node == "foraging_lore")
        );
        assert_eq!(
            colony
                .officers
                .get(&cat_sim::officers::OfficerRole::Accountant),
            Some(&cat_ids[0])
        );
        assert!(
            colony
                .jobs
                .iter()
                .any(|job| job.kind == SimJobKind::HaulGatherSpot)
        );
        assert!(
            colony
                .jobs
                .iter()
                .any(|job| job.kind == SimJobKind::CarryOffering)
        );
        assert!(colony.resources.food < 500.0);
        assert!(colony.resources.refined < 100.0);
        assert!(colony.global_upgrade_points > 0.0);
        assert_eq!(
            world.colonies[1], beta_before,
            "beta was mutated by alpha actions"
        );
    }

    #[tokio::test]
    async fn authenticated_join_routes_mutations_and_snapshots_to_selected_colony() {
        let state = build_state(1_000_000);
        let (mut connection, signed) = authenticated_connection(&state);
        let found = ClientAction::FoundVillage {
            name: "Beta".to_owned(),
            session_id: signed.session_id.clone(),
            sig: Some(signed.sig.clone()),
        };
        let found_result = send_action(&state, &mut connection, &found).await;
        assert!(found_result.result.ok, "{found_result:?}");
        let beta_id = found_result.result.colony_id.expect("personal village id");
        {
            let mut world = state.world.lock().await;
            let world_seed = world.world_seed;
            let beta_index = world
                .colonies
                .iter()
                .position(|colony| colony.id == beta_id)
                .expect("founded personal village");
            let beta = &mut world.colonies[beta_index];
            beta.upgrade_tree
                .owned_node_ids
                .push("basic_tools".to_owned());
            let workshop = beta
                .buildings
                .iter_mut()
                .find(|building| {
                    building.building_type == cat_sim::types::BuildingType::Woodworking
                })
                .expect("starter woodworking footprint");
            workshop.id = "beta-workshop".to_owned();
            workshop.building_type = cat_sim::types::BuildingType::Workshop;
            workshop.assigned_cat = None;
            workshop.automated_by = None;
            beta.resources.materials = beta.resources.materials.max(100.0);
            beta.officers.insert(
                cat_sim::officers::OfficerRole::Steward,
                beta.cats[0].id.clone(),
            );
            let _ = world_tick(&mut world, 1_060_000);
            let beta = &mut world.colonies[beta_index];
            beta.officers.clear();
            let workshop = beta
                .buildings
                .iter()
                .find(|building| building.id == "beta-workshop")
                .expect("beta workshop remains after access pass");
            assert!(cat_sim::world_tick::building_is_road_connected_to_shrine(
                beta, workshop, world_seed
            ));
        }
        connection.colony_id = STARTER_COLONY_ID.to_owned();
        let join = ClientAction::JoinVillage {
            colony_id: beta_id.clone(),
            session_id: signed.session_id.clone(),
            sig: Some(signed.sig.clone()),
        };
        assert!(send_action(&state, &mut connection, &join).await.result.ok);
        assert_eq!(connection.colony_id, beta_id);

        let beta_cat_id = state.world.lock().await.colonies[1].cats[0].id.clone();
        let assign = ClientAction::AssignOfficer {
            session_id: signed.session_id,
            nickname: "Tester".to_owned(),
            sig: signed.sig,
            role: OfficerRole::Steward,
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
                .get(&cat_sim::officers::OfficerRole::Steward),
            Some(&beta_cat_id)
        );
        drop(world);
        let snapshot = current_snapshot(&state, 1, &connection).await;
        assert_eq!(snapshot.colonies[0].id, beta_id);
        assert!(snapshot.colonies[0].capabilities.is_owner);
    }

    #[tokio::test]
    async fn strangers_never_receive_or_control_a_private_village() {
        let state = build_state(1_000_000);
        let (mut owner_connection, owner) = authenticated_connection(&state);
        let found = ClientAction::FoundVillage {
            name: "Secret Fern".to_owned(),
            session_id: owner.session_id.clone(),
            sig: Some(owner.sig.clone()),
        };
        let result = send_action(&state, &mut owner_connection, &found).await;
        let private_id = result.result.colony_id.expect("founded id");
        let anonymous =
            ConnectionContext::new("anonymous".to_owned(), STARTER_COLONY_ID.to_owned());
        let anonymous_snapshot = current_snapshot(&state, 1, &anonymous).await;
        let anonymous_json = serde_json::to_string(&anonymous_snapshot).expect("snapshot json");
        assert_eq!(anonymous_snapshot.colonies.len(), 1);
        assert!(!anonymous_json.contains("Secret Fern"));
        assert!(!anonymous_json.contains(&private_id));
        assert!(!anonymous_json.contains("ownerPlayerId"));

        let intruder = signed_session("intruder-session".to_owned(), state.session_secret.as_str());
        let mut intruder_connection =
            ConnectionContext::new("intruder".to_owned(), STARTER_COLONY_ID.to_owned());
        intruder_connection.identity = Some(intruder.clone());
        let intruder_snapshot = current_snapshot(&state, 1, &intruder_connection).await;
        assert_eq!(intruder_snapshot.colonies.len(), 1);
        let join = ClientAction::JoinVillage {
            colony_id: private_id,
            session_id: intruder.session_id,
            sig: Some(intruder.sig),
        };
        let denied = send_action(&state, &mut intruder_connection, &join).await;
        assert!(!denied.result.ok);
        assert_eq!(intruder_connection.colony_id, STARTER_COLONY_ID);

        let owner_snapshot = current_snapshot(&state, 1, &owner_connection).await;
        assert_eq!(owner_snapshot.colonies.len(), 2);
        assert!(owner_snapshot.colonies[0].capabilities.is_owner);
        assert!(owner_snapshot.colonies[0].capabilities.can_control);
    }

    #[tokio::test]
    async fn two_signed_players_found_discover_and_atomically_trade_between_villages() {
        let state = build_state(1_000_000);
        let first_identity =
            signed_session("first-player".to_owned(), state.session_secret.as_str());
        let second_identity =
            signed_session("second-player".to_owned(), state.session_secret.as_str());
        let mut first = ConnectionContext::new("first".to_owned(), STARTER_COLONY_ID.to_owned());
        first.identity = Some(first_identity.clone());
        let mut second = ConnectionContext::new("second".to_owned(), STARTER_COLONY_ID.to_owned());
        second.identity = Some(second_identity.clone());

        // The shared hub is genuinely communal: two unrelated signed players can
        // independently guide its economy before either founds a private village.
        for (connection, identity, kind) in [
            (
                &mut first,
                &first_identity,
                cat_protocol::JobKind::SupplyFood,
            ),
            (
                &mut second,
                &second_identity,
                cat_protocol::JobKind::SupplyWater,
            ),
        ] {
            let requested = send_action(
                &state,
                connection,
                &ClientAction::RequestJob {
                    session_id: identity.session_id.clone(),
                    nickname: "Commons Helper".to_owned(),
                    sig: identity.sig.clone(),
                    kind,
                },
            )
            .await;
            assert!(requested.result.ok, "signed communal request was denied");
        }

        let first_found = send_action(
            &state,
            &mut first,
            &ClientAction::FoundVillage {
                name: "Moss Rest".to_owned(),
                session_id: first_identity.session_id.clone(),
                sig: Some(first_identity.sig.clone()),
            },
        )
        .await;
        let first_id = first_found.result.colony_id.expect("first village");
        let second_found = send_action(
            &state,
            &mut second,
            &ClientAction::FoundVillage {
                name: "Reed Rest".to_owned(),
                session_id: second_identity.session_id.clone(),
                sig: Some(second_identity.sig.clone()),
            },
        )
        .await;
        let second_id = second_found.result.colony_id.expect("second village");
        assert_ne!(first_id, second_id);

        {
            let world = state.world.lock().await;
            let global = world
                .colonies
                .iter()
                .find(|colony| colony.kind == VillageKind::Global)
                .expect("canonical communal village");
            assert_eq!(global.scale, VillageScale::Communal);
            assert_eq!(global.owner_player_id, None);
            assert_eq!(global.jobs.len(), 2);
            assert_eq!(
                global
                    .cats
                    .iter()
                    .filter(|cat| cat.death_time.is_none())
                    .count(),
                30
            );
            assert_eq!(
                global
                    .buildings
                    .iter()
                    .filter(|building| {
                        building.building_type == cat_sim::types::BuildingType::Den
                    })
                    .count(),
                6
            );
            for id in [&first_id, &second_id] {
                let personal = world
                    .colonies
                    .iter()
                    .find(|colony| colony.id == *id)
                    .expect("signed personal village");
                assert_eq!(personal.scale, VillageScale::Personal);
                assert_eq!(
                    personal
                        .cats
                        .iter()
                        .filter(|cat| cat.death_time.is_none())
                        .count(),
                    15
                );
                assert_eq!(
                    personal
                        .buildings
                        .iter()
                        .filter(|building| {
                            building.building_type == cat_sim::types::BuildingType::Den
                        })
                        .count(),
                    3
                );
                assert_ne!(personal.stockpiles[0].rect, global.stockpiles[0].rect);
            }
        }

        // Model completed shrine-return exploration without bypassing any action
        // authorization: contact itself is simulation state, while both trade
        // mutations below traverse the real signed server handler.
        {
            let mut world = state.world.lock().await;
            let first_index = world
                .colonies
                .iter()
                .position(|colony| colony.id == first_id)
                .expect("first runtime");
            let second_index = world
                .colonies
                .iter()
                .position(|colony| colony.id == second_id)
                .expect("second runtime");
            let second_anchor = world.colonies[second_index].anchor;
            world.colonies[first_index]
                .pending_scout_delivery_tiles
                .insert(TilePos {
                    x: second_anchor.x + 1,
                    y: second_anchor.y + 1,
                });
            cat_sim::world_tick::reconcile_village_discoveries(&mut world);
            assert!(
                world.colonies[first_index]
                    .known_village_ids
                    .contains(&second_id)
            );
            assert!(
                world.colonies[second_index]
                    .known_village_ids
                    .contains(&first_id)
            );
            world.colonies[first_index].resources.food = 80.0;
            world.colonies[second_index].resources.materials = 80.0;
            let directory = village_directory(&world);
            let snapshot = build_snapshot(&world, 1_000_000, 2);
            drop(world);
            *state.village_directory.write().await = directory;
            *state.completed_snapshot.write().await = snapshot;
        }
        let first_snapshot = current_snapshot(&state, 2, &first).await;
        assert!(
            first_snapshot
                .known_villages
                .iter()
                .any(|village| village.id == second_id && !village.capabilities.can_view)
        );
        assert!(
            first_snapshot
                .colonies
                .iter()
                .all(|colony| colony.id != second_id),
            "discovery shares a map summary, never a foreign private simulation"
        );

        let proposed = send_action(
            &state,
            &mut first,
            &ClientAction::OfferVillageTrade {
                session_id: first_identity.session_id,
                nickname: "Moss Cat".to_owned(),
                sig: first_identity.sig,
                target_colony_id: second_id.clone(),
                offered_kind: cat_protocol::ResourceKind::Food,
                offered_amount: 12.0,
                requested_kind: cat_protocol::ResourceKind::Materials,
                requested_amount: 7.0,
            },
        )
        .await;
        assert!(proposed.result.ok, "{proposed:?}");
        let offer_id = state
            .world
            .lock()
            .await
            .colonies
            .iter()
            .find(|colony| colony.id == first_id)
            .and_then(|colony| colony.village_trade_offers.keys().next())
            .expect("signed trade offer")
            .clone();
        let accepted = send_action(
            &state,
            &mut second,
            &ClientAction::AcceptVillageTrade {
                session_id: second_identity.session_id,
                nickname: "Reed Cat".to_owned(),
                sig: second_identity.sig,
                offer_id,
            },
        )
        .await;
        assert!(accepted.result.ok, "{accepted:?}");

        let world = state.world.lock().await;
        let first_colony = world
            .colonies
            .iter()
            .find(|colony| colony.id == first_id)
            .expect("first colony");
        let second_colony = world
            .colonies
            .iter()
            .find(|colony| colony.id == second_id)
            .expect("second colony");
        assert_eq!(first_colony.resources.food, 68.0);
        assert_eq!(second_colony.resources.materials, 73.0);
        assert!(first_colony.resources.materials >= 67.0);
        assert!(second_colony.resources.food >= 62.0);
        assert!(first_colony.village_trade_offers.is_empty());
    }

    #[tokio::test]
    async fn restored_bearer_owns_the_same_village_across_connections() {
        let state = build_state(1_000_000);
        let mut first =
            ConnectionContext::new("first-socket".to_owned(), STARTER_COLONY_ID.to_owned());
        let issued = send_action(
            &state,
            &mut first,
            &ClientAction::Presence {
                session_id: "desktop".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: None,
            },
        )
        .await;
        let session_id = issued.fields.get("sessionId").expect("session id").clone();
        let sig = issued.fields.get("sig").expect("signature").clone();
        let player_id = issued.fields.get("playerId").expect("player id").clone();
        let found = send_action(
            &state,
            &mut first,
            &ClientAction::FoundVillage {
                name: "Persistent Fern".to_owned(),
                session_id: session_id.clone(),
                sig: Some(sig.clone()),
            },
        )
        .await;
        let village_id = found.result.colony_id.expect("personal village");

        let mut restored =
            ConnectionContext::new("second-socket".to_owned(), STARTER_COLONY_ID.to_owned());
        let restored_presence = send_action(
            &state,
            &mut restored,
            &ClientAction::Presence {
                session_id: session_id.clone(),
                nickname: "Desktop Cat".to_owned(),
                sig: Some(sig.clone()),
            },
        )
        .await;
        assert_eq!(restored_presence.fields.get("playerId"), Some(&player_id));
        let join = send_action(
            &state,
            &mut restored,
            &ClientAction::JoinVillage {
                colony_id: village_id.clone(),
                session_id,
                sig: Some(sig),
            },
        )
        .await;

        assert!(join.result.ok, "{join:?}");
        assert_eq!(restored.colony_id, village_id);
        let snapshot = current_snapshot(&state, 1, &restored).await;
        assert_eq!(snapshot.colonies[0].name, "Persistent Fern");
        assert!(snapshot.colonies[0].capabilities.is_owner);
    }

    #[tokio::test]
    async fn restored_bearer_owns_the_same_village_after_database_reload() {
        let path = std::env::temp_dir().join(format!(
            "cat-server-village-restart-{}-{}.db",
            std::process::id(),
            NEXT_DATABASE_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_file(&path);
        let secret = "restart-session-secret";
        let conn = Connection::open(&path).expect("open village database");
        persistence::init_schema(&conn).expect("init village database");
        let state = build_state_from_world(
            starter_world(1_000_000),
            conn,
            secret.to_owned(),
            false,
            1_000_000,
        );
        let mut first =
            ConnectionContext::new("first-socket".to_owned(), STARTER_COLONY_ID.to_owned());
        let issued = send_action(
            &state,
            &mut first,
            &ClientAction::Presence {
                session_id: "desktop".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: None,
            },
        )
        .await;
        let session_id = issued.fields.get("sessionId").expect("session id").clone();
        let sig = issued.fields.get("sig").expect("signature").clone();
        let found = send_action(
            &state,
            &mut first,
            &ClientAction::FoundVillage {
                name: "Restart Fern".to_owned(),
                session_id: session_id.clone(),
                sig: Some(sig.clone()),
            },
        )
        .await;
        let village_id = found.result.colony_id.expect("personal village");
        save_current_world(&state).await.expect("persist world");
        drop(state);

        let conn = Connection::open(&path).expect("reopen village database");
        persistence::init_schema(&conn).expect("migrate reopened database");
        let restarted =
            build_state_from_connection(2_000_000, conn, secret.to_owned()).expect("restore world");
        let mut restored =
            ConnectionContext::new("restored-socket".to_owned(), STARTER_COLONY_ID.to_owned());
        let presence = send_action(
            &restarted,
            &mut restored,
            &ClientAction::Presence {
                session_id: session_id.clone(),
                nickname: "Desktop Cat".to_owned(),
                sig: Some(sig.clone()),
            },
        )
        .await;
        assert!(presence.result.ok, "{presence:?}");
        let join = send_action(
            &restarted,
            &mut restored,
            &ClientAction::JoinVillage {
                colony_id: village_id.clone(),
                session_id,
                sig: Some(sig),
            },
        )
        .await;
        assert!(join.result.ok, "{join:?}");
        assert_eq!(restored.colony_id, village_id);
        let snapshot = current_snapshot(&restarted, 1, &restored).await;
        assert_eq!(snapshot.colonies[0].name, "Restart Fern");
        assert!(snapshot.colonies[0].capabilities.is_owner);

        drop(restarted);
        fs::remove_file(path).expect("remove village database");
    }

    #[tokio::test]
    async fn signed_vote_kick_survives_restart_and_counts_each_reconnected_player_once() {
        let path = std::env::temp_dir().join(format!(
            "cat-server-vote-kick-restart-{}-{}.db",
            std::process::id(),
            NEXT_DATABASE_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_file(&path);
        let secret = "vote-kick-restart-secret";
        let mut world = starter_world(1_000_000);
        let original_leader = world.colonies[0].cats[0].id.clone();
        world.colonies[0].leader_id = Some(original_leader.clone());
        let conn = Connection::open(&path).expect("open vote-kick database");
        persistence::init_schema(&conn).expect("init vote-kick database");
        let state = build_state_from_world(world, conn, secret.to_owned(), false, 1_000_000);

        let mut issued_sessions = Vec::new();
        for voter in 0..3 {
            let mut connection = ConnectionContext::new(
                format!("voter-{voter}-socket"),
                STARTER_COLONY_ID.to_owned(),
            );
            let presence = send_action(
                &state,
                &mut connection,
                &ClientAction::Presence {
                    session_id: format!("voter-{voter}"),
                    nickname: format!("Voter {voter}"),
                    sig: None,
                },
            )
            .await;
            let session_id = presence.fields["sessionId"].clone();
            let sig = presence.fields["sig"].clone();
            let result = send_action(
                &state,
                &mut connection,
                &ClientAction::RequestVoteKick {
                    session_id: session_id.clone(),
                    nickname: format!("Voter {voter}"),
                    sig: sig.clone(),
                },
            )
            .await;
            assert!(result.result.ok, "signed petition failed: {result:?}");
            issued_sessions.push((session_id, sig));
        }
        {
            let world = state.world.lock().await;
            assert_eq!(
                build_snapshot(&world, now_ms(), 0).colonies[0]
                    .vote_kick
                    .as_ref()
                    .expect("open petition")
                    .signatures,
                3
            );
        }
        save_current_world(&state).await.expect("persist petition");
        drop(state);

        let conn = Connection::open(&path).expect("reopen vote-kick database");
        persistence::init_schema(&conn).expect("migrate vote-kick database");
        let restarted = build_state_from_connection(2_000_000, conn, secret.to_owned())
            .expect("restore petition");

        // Reconnect the first bearer and repeat its action: the stable player id
        // restores, but the petition still has exactly one signature from it.
        let (session_id, sig) = issued_sessions[0].clone();
        let mut reconnected = ConnectionContext::new(
            "voter-0-reconnected".to_owned(),
            STARTER_COLONY_ID.to_owned(),
        );
        let presence = send_action(
            &restarted,
            &mut reconnected,
            &ClientAction::Presence {
                session_id: session_id.clone(),
                nickname: "Voter 0".to_owned(),
                sig: Some(sig.clone()),
            },
        )
        .await;
        assert!(presence.result.ok, "bearer reconnect failed: {presence:?}");
        let duplicate = send_action(
            &restarted,
            &mut reconnected,
            &ClientAction::RequestVoteKick {
                session_id,
                nickname: "Voter 0".to_owned(),
                sig,
            },
        )
        .await;
        assert!(
            duplicate.result.ok,
            "idempotent repeat failed: {duplicate:?}"
        );
        assert_eq!(restarted.world.lock().await.colonies[0].votes.len(), 3);

        for voter in 3..5 {
            let mut connection = ConnectionContext::new(
                format!("voter-{voter}-socket"),
                STARTER_COLONY_ID.to_owned(),
            );
            let presence = send_action(
                &restarted,
                &mut connection,
                &ClientAction::Presence {
                    session_id: format!("voter-{voter}"),
                    nickname: format!("Voter {voter}"),
                    sig: None,
                },
            )
            .await;
            let result = send_action(
                &restarted,
                &mut connection,
                &ClientAction::RequestVoteKick {
                    session_id: presence.fields["sessionId"].clone(),
                    nickname: format!("Voter {voter}"),
                    sig: presence.fields["sig"].clone(),
                },
            )
            .await;
            assert!(result.result.ok, "signed petition failed: {result:?}");
        }

        let closes_at = {
            let world = restarted.world.lock().await;
            let colony = &world.colonies[0];
            assert_eq!(colony.votes.len(), 5);
            let petition = colony
                .elections
                .iter()
                .find(|election| {
                    election.kind == cat_sim::world_tick::ElectionKind::VoteKick
                        && election.resolved_at.is_none()
                })
                .expect("restored open petition");
            assert_eq!(
                petition.winner_cat_id.as_deref(),
                Some(original_leader.as_str())
            );
            petition.closes_at
        };
        {
            let mut world = restarted.world.lock().await;
            let _ = world_tick(&mut world, closes_at);
            assert_ne!(
                world.colonies[0].leader_id.as_deref(),
                Some(original_leader.as_str())
            );
        }
        save_current_world(&restarted)
            .await
            .expect("persist resolved petition");
        drop(restarted);

        let conn = Connection::open(&path).expect("reopen resolved database");
        persistence::init_schema(&conn).expect("migrate resolved database");
        let final_state = build_state_from_connection(3_000_000, conn, secret.to_owned())
            .expect("restore resolved petition");
        assert_ne!(
            final_state.world.lock().await.colonies[0]
                .leader_id
                .as_deref(),
            Some(original_leader.as_str())
        );
        drop(final_state);
        fs::remove_file(path).expect("remove vote-kick database");
    }

    #[tokio::test]
    async fn signed_exact_building_click_rejects_collisions_and_restores_completed_site() {
        let path = std::env::temp_dir().join(format!(
            "cat-server-exact-build-restart-{}-{}.db",
            std::process::id(),
            NEXT_DATABASE_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_file(&path);
        let secret = "exact-build-restart-secret";
        let started_at = now_ms();
        let mut world = starter_world(started_at);
        let colony = &mut world.colonies[0];
        colony.resources.lumber = 40.0;
        colony.resources.planks = 40.0;
        colony.resources.blocks = 40.0;
        cat_sim::world_tick::reconcile_colony_stockpiles(colony);
        colony.officers.insert(
            cat_sim::officers::OfficerRole::Steward,
            colony.cats[0].id.clone(),
        );
        let mut claimed = colony.claimed_tiles.clone();
        claimed.sort_by_key(|tile| (tile.y, tile.x));
        let site = claimed
            .into_iter()
            .find(|site| {
                cat_sim::world_tick::can_plan_building_at(
                    colony,
                    *site,
                    WORLD_SEED,
                    cat_sim::types::BuildingType::WaterBowl,
                )
            })
            .expect("founding claim has a clickable bowl site");
        let conn = Connection::open(&path).expect("open exact-build database");
        persistence::init_schema(&conn).expect("init exact-build database");
        let state = build_state_from_world(world, conn, secret.to_owned(), true, started_at);

        let mut socket =
            ConnectionContext::new("builder-socket".to_owned(), STARTER_COLONY_ID.to_owned());
        let presence = send_action(
            &state,
            &mut socket,
            &ClientAction::Presence {
                session_id: "exact-builder".to_owned(),
                nickname: "Builder".to_owned(),
                sig: None,
            },
        )
        .await;
        let session_id = presence.fields["sessionId"].clone();
        let sig = presence.fields["sig"].clone();
        let signed_plan = |building_type, site| ClientAction::PlanBuilding {
            session_id: session_id.clone(),
            nickname: "Builder".to_owned(),
            sig: sig.clone(),
            building_type,
            site: Some(site),
        };

        let before = state.world.lock().await.colonies[0].clone();
        let outside = send_action(
            &state,
            &mut socket,
            &signed_plan(
                cat_protocol::BuildingType::WaterBowl,
                TilePoint {
                    x: site.x + 100_000,
                    y: site.y + 100_000,
                },
            ),
        )
        .await;
        assert!(!outside.result.ok);
        {
            let world = state.world.lock().await;
            assert_eq!(world.colonies[0].resources, before.resources);
            assert_eq!(world.colonies[0].buildings, before.buildings);
        }

        let accepted = send_action(
            &state,
            &mut socket,
            &signed_plan(
                cat_protocol::BuildingType::WaterBowl,
                TilePoint {
                    x: site.x,
                    y: site.y,
                },
            ),
        )
        .await;
        assert!(accepted.result.ok, "exact build failed: {accepted:?}");
        let paid = state.world.lock().await.colonies[0].clone();
        let scaffold = paid.buildings.last().expect("exact scaffold");
        assert_eq!(scaffold.position, site);
        assert!(!scaffold.is_complete);
        assert_eq!(paid.resources, before.resources);
        assert!(scaffold.construction_cargo.is_some());
        {
            let world = state.world.lock().await;
            let snapshot = build_snapshot(&world, started_at, 1);
            let physical = snapshot.colonies[0]
                .buildings
                .iter()
                .find(|building| building.id == scaffold.id)
                .expect("new scaffold is inspectable");
            assert!(!physical.construction_required.is_empty());
            assert!(physical.construction_delivered.is_empty());
            assert!(physical.construction_in_transit.is_empty());
            assert!(physical.construction_block_reason.is_some());
        }

        let overlap = send_action(
            &state,
            &mut socket,
            &signed_plan(
                cat_protocol::BuildingType::Walls,
                TilePoint {
                    x: site.x,
                    y: site.y,
                },
            ),
        )
        .await;
        assert!(!overlap.result.ok);
        {
            let world = state.world.lock().await;
            assert_eq!(world.colonies[0].resources, paid.resources);
            assert_eq!(world.colonies[0].buildings, paid.buildings);
        }

        let completed_id = scaffold.id.clone();
        {
            let mut world = state.world.lock().await;
            world.colonies[0]
                .jobs
                .iter_mut()
                .find(|job| {
                    matches!(
                        &job.metadata,
                        cat_sim::world_tick::JobMetadata::Construction {
                            building_id: Some(id),
                            ..
                        } if id == &completed_id
                    )
                })
                .expect("signed scaffold owns one construction job")
                .duration_ms = 10_000;
            let mut tick_at = started_at + 1_000;
            for _ in 0..1_200 {
                let _ = world_tick(&mut world, tick_at);
                if world.colonies[0]
                    .buildings
                    .iter()
                    .find(|building| building.id == completed_id)
                    .is_some_and(|building| building.is_complete)
                {
                    break;
                }
                tick_at = world.colonies[0]
                    .jobs
                    .iter()
                    .find(|job| {
                        matches!(
                            &job.metadata,
                            cat_sim::world_tick::JobMetadata::Construction {
                                building_id: Some(id),
                                ..
                            } if id == &completed_id
                        )
                    })
                    .and_then(|job| job.ends_at)
                    .filter(|ends_at| *ends_at > tick_at)
                    .unwrap_or(tick_at + 1_000);
            }
            let building = world.colonies[0]
                .buildings
                .iter()
                .find(|building| building.id == completed_id)
                .unwrap_or_else(|| {
                    panic!(
                        "exact building survives completion: run={} ids={:?} events={:?}",
                        world.colonies[0].run_number,
                        world.colonies[0]
                            .buildings
                            .iter()
                            .map(|building| building.id.as_str())
                            .collect::<Vec<_>>(),
                        world.colonies[0]
                            .events
                            .iter()
                            .rev()
                            .take(4)
                            .map(|event| event.message.as_str())
                            .collect::<Vec<_>>()
                    )
                });
            assert!(
                building.is_complete,
                "physical construction stalled: building={building:?} job={:?} builders={:?} resources={:?} piles={:?}",
                world.colonies[0].jobs.iter().find(|job| {
                    matches!(
                        &job.metadata,
                        cat_sim::world_tick::JobMetadata::Construction {
                            building_id: Some(id),
                            ..
                        } if id == &completed_id
                    )
                }),
                world.colonies[0]
                    .cats
                    .iter()
                    .filter(|cat| cat.current_task == Some(cat_sim::types::TaskType::Build))
                    .collect::<Vec<_>>(),
                world.colonies[0].resources,
                world.colonies[0]
                    .stockpiles
                    .iter()
                    .map(|pile| (&pile.id, &pile.contents))
                    .collect::<Vec<_>>()
            );
            assert_eq!(building.position, site);
        }
        save_current_world(&state)
            .await
            .expect("persist exact building");
        drop(state);

        let conn = Connection::open(&path).expect("reopen exact-build database");
        persistence::init_schema(&conn).expect("migrate exact-build database");
        let restarted = build_state_from_connection(started_at + 2_000, conn, secret.to_owned())
            .expect("restore exact building");
        let restored = restarted.world.lock().await.colonies[0]
            .buildings
            .iter()
            .find(|building| building.id == completed_id)
            .cloned()
            .expect("completed clicked building restored");
        assert!(restored.is_complete);
        assert_eq!(restored.position, site);

        let mut reconnected = ConnectionContext::new(
            "builder-reconnected".to_owned(),
            STARTER_COLONY_ID.to_owned(),
        );
        let presence = send_action(
            &restarted,
            &mut reconnected,
            &ClientAction::Presence {
                session_id,
                nickname: "Builder".to_owned(),
                sig: Some(sig),
            },
        )
        .await;
        assert!(
            presence.result.ok,
            "builder bearer reconnects: {presence:?}"
        );
        drop(restarted);
        fs::remove_file(path).expect("remove exact-build database");
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
