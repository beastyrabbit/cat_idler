//! Axum server shell for the Cat Colony simulation, porting the transport around
//! `server/game.ts:workerTick` and `app/api/game/actions/route.ts`.

use std::{
    collections::{BTreeMap, BTreeSet},
    net::{IpAddr, SocketAddr},
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicU32, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use axum::{
    Extension, Router,
    body::Body,
    extract::{
        ConnectInfo, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{
        HeaderMap, Request, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS},
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use cat_protocol::lai64::{
    ActionErrorSnapshot, ActionOutcome, ActionReceipt, CanonicalActionEnvelope,
    CanonicalSnapshotEnvelope, CanonicalWireError, PublicColonySummaryV2, ReportText, StableId,
    VersionExpectation, VersionLane,
};
use cat_protocol::{
    ActionAcceptedResult, ActionProtocolVersion, ActionResult, BoundedEntityId, ClientAction,
    CurrentStateHint, CurrentVersionHint, LeaderAiActionEnvelope, LeaderAiActionResponse,
    LeaderAiActionResult, ReportSafeString, WorldSnapshot,
};
#[cfg(test)]
use cat_protocol::{VillageCapabilities, VillageKind as ProtocolVillageKind};
use cat_sim::{
    actions::{ActionCtx, apply_action, build_snapshot},
    world_tick::{
        ColonyRuntime, EventKind, EventLog, TilePos, VillageKind, VillageScale, WorldState,
        found_global_colony, new_world, register_colony_spatial, world_tick,
    },
};
use hosting::ServerConfig;
use identity::{
    SESSION_MAX_AGE_MS, SignedSession, issue_session, renew_session_at, session_issued_at,
    signed_session, verify_session, verify_session_at,
};
use leader_ai_action_routing::{
    AtomicLeaderAiCommit, ColonyControlPolicy, IdempotencyReceiptStore, IdempotencyReplay,
    LeaderAiServerMutationPipeline, NoMutationBeforePreconditions, OrderedMutationExecutor,
    OwnsSelectedColony, SelectedColonyOwnershipGuard, SelectedColonyOwnershipSource,
    ServerActionConflict, ServerActionResult as LeaderAiServerActionResult, ServerMutationActor,
    VerifiedPlayerSession, check_actor_action_authority, check_expected_state_versions,
    check_hmac_session_authentication, check_protocol_compatibility,
    check_selected_colony_ownership, current_action_protocol_version, decode_lai_action_envelope,
    minimum_supported_action_protocol_version, project_server_action_response,
    reject_before_action_decode,
};
use persistence::{
    load_world, open_database_from_env, save_world, save_world_with_canonical_boundary,
};
use rate_limit::RateLimiter;
use rusqlite::Connection;
use tokio::sync::{Mutex, RwLock, broadcast};
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
};
use tracing::{debug, error, info, warn};

#[cfg(test)]
use std::sync::atomic::AtomicU64;

mod hosting;
mod identity;
pub mod lai65;
pub mod leader_ai_action_routing;
pub mod leader_ai_persistence;
mod leader_ai_snapshot_projection;
mod persistence;
mod rate_limit;

const WORLD_SEED: u32 = 20_240_703;
const STARTER_COLONY_ID: &str = "colony-1";
const STARTER_COLONY_SEED: u32 = 1;
const SNAPSHOT_CHANNEL_CAPACITY: usize = 32;
const ACTION_LIMIT_MAX: usize = 30;
const ACTION_LIMIT_WINDOW_MS: i64 = 10_000;
const IP_ACTION_LIMIT_MAX: usize = 120;
const SESSION_ISSUE_LIMIT_MAX: usize = 8;
const SESSION_ISSUE_LIMIT_WINDOW_MS: i64 = 60 * 60 * 1_000;
const MAX_CONNECTIONS_PER_IP: usize = 8;
const MAX_PERSONAL_VILLAGES_PER_IP: usize = 8;
const MAX_TOTAL_COLONIES: usize = 256;
const MAX_WEBSOCKET_MESSAGE_BYTES: usize = 64 * 1_024;
const SAVE_FAILURES_BEFORE_NOT_READY: u32 = 3;
const SAVE_EVERY_TICKS: u64 = 5;
const TEST_ACTIONS_ENV: &str = "CAT_SERVER_ENABLE_TEST_ACTIONS";
const BROWSER_FIXTURE_FREEZE_ENV: &str = "CAT_SERVER_BROWSER_FIXTURE_FREEZE";
const PLAYER_NAME_MAX_CHARS: usize = 24;

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
    /// Schema-v2 canonical action replay rows.  This is intentionally separate
    /// from the retired LAI.24/25 receipt shape: a canonical request is never
    /// translated into an old action before authorization.
    canonical_replay: Arc<Mutex<lai65::CanonicalReplayStore>>,
    canonical_hole_clicks: Arc<Mutex<lai65::HoleClickRateLimiter>>,
    canonical_test_reset: Arc<Mutex<lai65::TwoStepSignedTestResetGate>>,
    ip_rate_limiter: Arc<Mutex<RateLimiter>>,
    abuse_guard: Arc<Mutex<AbuseGuard>>,
    peer_connections: Arc<PeerConnections>,
    consecutive_save_failures: Arc<AtomicU32>,
    session_secret: Arc<String>,
    allow_test_actions: bool,
}

#[derive(Debug)]
struct AbuseGuard {
    session_issuance: RateLimiter,
    player_peers: BTreeMap<String, String>,
}

#[derive(Debug, Default)]
struct PeerConnections {
    counts: StdMutex<BTreeMap<IpAddr, usize>>,
}

impl PeerConnections {
    fn acquire(self: &Arc<Self>, peer: IpAddr) -> Option<PeerConnectionGuard> {
        let mut counts = self
            .counts
            .lock()
            .expect("peer connection registry poisoned");
        let count = counts.entry(peer).or_default();
        if *count >= MAX_CONNECTIONS_PER_IP {
            return None;
        }
        *count += 1;
        Some(PeerConnectionGuard {
            registry: Arc::clone(self),
            peer,
        })
    }
}

struct PeerConnectionGuard {
    registry: Arc<PeerConnections>,
    peer: IpAddr,
}

impl Drop for PeerConnectionGuard {
    fn drop(&mut self) {
        let mut counts = self
            .registry
            .counts
            .lock()
            .expect("peer connection registry poisoned");
        if let Some(count) = counts.get_mut(&self.peer) {
            *count = count.saturating_sub(1);
            debug!(
                peer_ip = %self.peer,
                remaining_connections = *count,
                "released WebSocket peer connection"
            );
            if *count == 0 {
                counts.remove(&self.peer);
            }
        }
    }
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

    let config = ServerConfig::from_env()?;
    let session_secret = identity::session_secret_from_env(config.listen_addr.ip())?;
    let conn = open_database_from_env()?;
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
    axum::serve(
        listener,
        app(state.clone(), &config).into_make_service_with_connect_info::<SocketAddr>(),
    )
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

    router
        .layer(Extension(config.trusted_proxies.clone()))
        .layer(CompressionLayer::new().br(true).gzip(true))
}

async fn health() -> &'static str {
    "ok"
}

async fn readiness(State(state): State<AppState>) -> impl IntoResponse {
    let database_ready = state.db.try_lock().is_ok_and(|db| {
        db.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
            .is_ok_and(|value| value == 1)
    });
    let world_ready = state
        .world
        .try_lock()
        .is_ok_and(|world| !world.colonies.is_empty());

    let persistence_ready =
        state.consecutive_save_failures.load(Ordering::SeqCst) < SAVE_FAILURES_BEFORE_NOT_READY;

    if database_ready && world_ready && persistence_ready {
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

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Extension(trusted_proxies): Extension<hosting::TrustedProxies>,
    headers: HeaderMap,
) -> Response {
    let peer_ip = match effective_peer_ip(peer.ip(), &headers, &trusted_proxies) {
        Ok(peer_ip) => peer_ip,
        Err(message) => {
            warn!(socket_peer = %peer.ip(), "rejected malformed trusted-proxy forwarding header");
            return (StatusCode::BAD_REQUEST, message).into_response();
        }
    };
    let Some(connection_guard) = state.peer_connections.acquire(peer_ip) else {
        warn!(
            %peer_ip,
            max_connections = MAX_CONNECTIONS_PER_IP,
            "rejected WebSocket peer connection limit"
        );
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many connections from this address.",
        )
            .into_response();
    };
    ws.max_message_size(MAX_WEBSOCKET_MESSAGE_BYTES)
        .max_frame_size(MAX_WEBSOCKET_MESSAGE_BYTES)
        .on_upgrade(move |socket| handle_socket(socket, state, peer_ip, connection_guard))
        .into_response()
}

fn effective_peer_ip(
    socket_peer: IpAddr,
    headers: &HeaderMap,
    trusted_proxies: &hosting::TrustedProxies,
) -> Result<IpAddr, &'static str> {
    if !trusted_proxies.contains(&socket_peer) {
        // Forwarding headers from ordinary clients are untrusted input and deliberately ignored.
        return Ok(socket_peer);
    }
    let mut forwarded_values = headers.get_all("x-forwarded-for").iter();
    let raw = forwarded_values
        .next()
        .ok_or("Trusted proxy did not provide X-Forwarded-For.")?
        .to_str()
        .map_err(|_| "Trusted proxy provided an invalid X-Forwarded-For header.")?;
    if forwarded_values.next().is_some() {
        return Err("Trusted proxy must provide one X-Forwarded-For header.");
    }
    if raw.contains(',') {
        // Require the edge proxy to overwrite rather than append. Accepting a chain without
        // configuring every hop makes the leftmost address client-controlled.
        return Err("Trusted proxy must provide exactly one client IP.");
    }
    raw.trim()
        .parse::<IpAddr>()
        .map_err(|_| "Trusted proxy provided an invalid client IP.")
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
    register_colony_spatial(&mut world, 0);
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
    let mut canonical_replay = lai65::CanonicalReplayStore::default();
    for row in leader_ai_persistence::load_canonical_replay_rows(&conn)? {
        canonical_replay.restore(row).map_err(|_| {
            rusqlite::Error::InvalidParameterName("invalid canonical replay state".to_owned())
        })?;
    }
    let mut canonical_hole_clicks = lai65::HoleClickRateLimiter::default();
    for row in leader_ai_persistence::load_canonical_hole_rate_rows(&conn)? {
        canonical_hole_clicks.restore(row).map_err(|_| {
            rusqlite::Error::InvalidParameterName("invalid canonical Hole-rate state".to_owned())
        })?;
    }
    let mut canonical_test_reset = lai65::TwoStepSignedTestResetGate::default();
    for row in leader_ai_persistence::load_canonical_test_reset_rows(&conn)? {
        if row.expires_at_ms < now_ms {
            continue;
        }
        canonical_test_reset
            .restore_staged_challenge(lai65::StagedTestResetChallenge {
                session_id: row.session_id,
                authenticated_player_id: row.authenticated_player_id,
                selected_colony_id: Some(row.selected_colony_id),
                stage_idempotency_id: Some(row.stage_idempotency_id),
                nonce: row.nonce,
                signature: row.signature,
                expires_at_ms: row.expires_at_ms,
            })
            .map_err(|_| {
                rusqlite::Error::InvalidParameterName(
                    "invalid canonical test-reset state".to_owned(),
                )
            })?;
    }
    Ok(build_state_from_world_with_boundary(
        world,
        conn,
        session_secret,
        test_actions_enabled(),
        now_ms,
        canonical_replay,
        canonical_hole_clicks,
        canonical_test_reset,
    ))
}

fn build_state_from_world(
    world: WorldState,
    conn: Connection,
    session_secret: String,
    allow_test_actions: bool,
    now_ms: i64,
) -> AppState {
    build_state_from_world_with_boundary(
        world,
        conn,
        session_secret,
        allow_test_actions,
        now_ms,
        lai65::CanonicalReplayStore::default(),
        lai65::HoleClickRateLimiter::default(),
        lai65::TwoStepSignedTestResetGate::default(),
    )
}

fn build_state_from_world_with_boundary(
    world: WorldState,
    conn: Connection,
    session_secret: String,
    allow_test_actions: bool,
    now_ms: i64,
    canonical_replay: lai65::CanonicalReplayStore,
    canonical_hole_clicks: lai65::HoleClickRateLimiter,
    canonical_test_reset: lai65::TwoStepSignedTestResetGate,
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
        canonical_replay: Arc::new(Mutex::new(canonical_replay)),
        canonical_hole_clicks: Arc::new(Mutex::new(canonical_hole_clicks)),
        canonical_test_reset: Arc::new(Mutex::new(canonical_test_reset)),
        ip_rate_limiter: Arc::new(Mutex::new(RateLimiter::new(
            IP_ACTION_LIMIT_MAX,
            ACTION_LIMIT_WINDOW_MS,
        ))),
        abuse_guard: Arc::new(Mutex::new(AbuseGuard {
            session_issuance: RateLimiter::new(
                SESSION_ISSUE_LIMIT_MAX,
                SESSION_ISSUE_LIMIT_WINDOW_MS,
            ),
            player_peers: BTreeMap::new(),
        })),
        peer_connections: Arc::new(PeerConnections::default()),
        consecutive_save_failures: Arc::new(AtomicU32::new(0)),
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

/// Keep a committed deterministic browser fixture stable while still routing
/// every signed user action through the real server. Debug builds only: a
/// release server cannot disable authoritative simulation ticks.
#[cfg(debug_assertions)]
fn browser_fixture_freeze_enabled() -> bool {
    std::env::var(BROWSER_FIXTURE_FREEZE_ENV)
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

#[cfg(not(debug_assertions))]
fn browser_fixture_freeze_enabled() -> bool {
    false
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
    if browser_fixture_freeze_enabled() {
        warn!(
            environment = BROWSER_FIXTURE_FREEZE_ENV,
            "authoritative simulation ticks frozen for deterministic browser fixture"
        );
        return;
    }
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
            match save_world(&db, &world) {
                Ok(()) => {
                    worker_state
                        .consecutive_save_failures
                        .store(0, Ordering::SeqCst);
                }
                Err(err) => {
                    let failures = worker_state
                        .consecutive_save_failures
                        .fetch_add(1, Ordering::SeqCst)
                        .saturating_add(1);
                    error!(%err, failures, "periodic world save failed");
                }
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
    let world = state.world.lock().await.clone();
    let db = state.db.lock().await;
    save_world(&db, &world)
}

async fn handle_socket(
    mut socket: WebSocket,
    state: AppState,
    peer_ip: IpAddr,
    _connection_guard: PeerConnectionGuard,
) {
    let directory = state.village_directory.read().await;
    let global_id = global_village_id(&directory);
    drop(directory);
    let mut connection = ConnectionContext::for_peer_ip(peer_ip, global_id);
    let _online_count = state.online_count.fetch_add(1, Ordering::SeqCst) + 1;
    let mut snapshots = state.snapshots.subscribe();

    // The production cutover waits for the signed Presence handshake before
    // choosing a private selected-colony projection. Unit tests retain the old
    // initial frame solely while legacy behavior is being removed from fixtures.
    #[cfg(test)]
    if send_current_snapshot(&mut socket, &state, _online_count, &connection)
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
                    Ok(_snapshot) => {
                        if connection.leader_ai_protocol && connection.identity.is_some() {
                            if send_leader_ai_snapshot(&mut socket, &state, &connection)
                                .await
                                .is_err()
                            {
                                break;
                            }
                        } else {
                            #[cfg(test)]
                            {
                            let directory = state.village_directory.read().await;
                            let snapshot = project_snapshot(
                                _snapshot,
                                &directory,
                                connection.identity.as_ref(),
                                &connection.colony_id,
                            );
                            drop(directory);
                            if send_snapshot(&mut socket, &snapshot).await.is_err() {
                                break;
                            }
                            }
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
                        // Authentication establishes the report boundary and a
                        // committed action may change it. Do not make the client
                        // wait for a future world tick: frozen deterministic
                        // fixtures have no such tick, and normal play otherwise
                        // shows a needless one-tick stale start/action surface.
                        if connection.leader_ai_protocol
                            && connection.identity.is_some()
                            && send_leader_ai_snapshot(&mut socket, &state, &connection)
                                .await
                                .is_err()
                        {
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

#[cfg(test)]
async fn send_current_snapshot(
    socket: &mut WebSocket,
    state: &AppState,
    online_count: u32,
    connection: &ConnectionContext,
) -> Result<(), axum::Error> {
    let snapshot = current_snapshot(state, online_count, connection).await;
    send_snapshot(socket, &snapshot).await
}

#[cfg(test)]
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
#[cfg(test)]
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
    }

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

    for colony in &mut snapshot.colonies {
        let expose_exact_equipment = selected.as_deref() == Some(colony.id.as_str())
            && colony.capabilities.can_control
            && colony
                .stock_ledger
                .as_ref()
                .is_some_and(|ledger| ledger.accurate);
        project_reported_stock(colony, expose_exact_equipment);
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
    snapshot.village_trade_caravans.retain(|caravan| {
        identity.is_some()
            && (controlled_ids.contains(&caravan.from_colony_id)
                || controlled_ids.contains(&caravan.to_colony_id))
    });

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
#[cfg(test)]
fn project_reported_stock(colony: &mut cat_protocol::ColonySnapshot, expose_exact_equipment: bool) {
    if !expose_exact_equipment {
        redact_exact_functional_equipment(colony);
    }

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

/// Functional equipment is finite, so its stack count, unit locations, loadout ids, and
/// carried ids are all authoritative stock facts. Only the selected colony's signed controller
/// may receive those facts, and only while the Accountant's canonical report is still exact.
/// Reported scalar tool/weapon/armor totals remain available through `resources`.
#[cfg(test)]
fn redact_exact_functional_equipment(colony: &mut cat_protocol::ColonySnapshot) {
    colony
        .items
        .retain(|stack| !matches!(stack.kind.as_str(), "tool" | "weapon" | "armor"));
    for cat in &mut colony.cats {
        cat.equipment = cat_protocol::EquipmentLoadoutSnapshot::default();
        if let Some(carrying) = &mut cat.carrying {
            carrying.item_ids.clear();
        }
    }
    if let Some(trader) = &mut colony.trader {
        trader
            .buy_offers
            .retain(|offer| !matches!(offer.kind.as_str(), "tool" | "weapon" | "armor"));
    }
}

/// Keep the socket-selected colony first because the current client renders the
/// first colony while retaining the complete shared-world snapshot for world-map
/// features.
#[cfg(test)]
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
    peer_ip: Option<IpAddr>,
    identity: Option<SignedSession>,
    /// Authenticated display name selected by the player for this connection.
    /// Player/install ids never enter snapshots or player-facing logs.
    nickname: Option<String>,
    colony_id: String,
    leader_ai_protocol: bool,
}

impl ConnectionContext {
    #[cfg(test)]
    fn new(limiter_fallback: String, global_colony_id: String) -> Self {
        Self {
            limiter_fallback,
            peer_ip: None,
            identity: None,
            nickname: None,
            colony_id: global_colony_id,
            leader_ai_protocol: false,
        }
    }

    fn for_peer_ip(peer_ip: IpAddr, global_colony_id: String) -> Self {
        Self {
            limiter_fallback: peer_ip.to_string(),
            peer_ip: Some(peer_ip),
            identity: None,
            nickname: None,
            colony_id: global_colony_id,
            leader_ai_protocol: false,
        }
    }

    fn limiter_key(&self) -> String {
        self.identity.as_ref().map_or_else(
            || format!("ip:{}", self.limiter_fallback),
            |identity| format!("s:{}", identity.session_id),
        )
    }

    fn peer_limiter_key(&self) -> String {
        format!("ip:{}", self.limiter_fallback)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ServerActionResult {
    result: ActionResult,
    fields: BTreeMap<&'static str, String>,
    leader_ai: Option<LeaderAiServerActionResult>,
    canonical: Option<CanonicalActionResponse>,
}

#[derive(Debug, Clone)]
enum CanonicalActionResponse {
    Receipt(ActionReceipt),
    Error(ActionErrorSnapshot),
}

impl ServerActionResult {
    fn from_result(result: ActionResult) -> Self {
        Self {
            result,
            fields: BTreeMap::new(),
            leader_ai: None,
            canonical: None,
        }
    }

    fn from_leader_ai(result: LeaderAiServerActionResult) -> Self {
        Self {
            result: ActionResult {
                ok: matches!(
                    result,
                    LeaderAiServerActionResult::Action(ref response)
                        if matches!(response.result, LeaderAiActionResult::Accepted { .. }
                            | LeaderAiActionResult::DuplicateReplay { .. })
                ),
                message: None,
                colony_id: None,
            },
            fields: BTreeMap::new(),
            leader_ai: Some(result),
            canonical: None,
        }
    }

    fn from_canonical(result: CanonicalActionResponse) -> Self {
        let accepted = matches!(
            result,
            CanonicalActionResponse::Receipt(ActionReceipt {
                outcome: ActionOutcome::Accepted | ActionOutcome::Replayed,
                ..
            })
        );
        Self {
            result: ActionResult {
                ok: accepted,
                message: None,
                colony_id: None,
            },
            fields: BTreeMap::new(),
            leader_ai: None,
            canonical: Some(result),
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
        if let Some(result) = &self.leader_ai {
            return match result {
                LeaderAiServerActionResult::UpdateRequired(response) => {
                    serde_json::to_string(response)
                }
                LeaderAiServerActionResult::ProtocolError(conflict) => {
                    serde_json::to_string(conflict)
                }
                LeaderAiServerActionResult::Action(response) => serde_json::to_string(response),
            };
        }
        if let Some(result) = &self.canonical {
            return match result {
                CanonicalActionResponse::Receipt(receipt) => serde_json::to_string(receipt),
                CanonicalActionResponse::Error(error) => serde_json::to_string(error),
            };
        }
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

/// The one production classification of the legacy `ClientAction` union.
///
/// `true` means the value is a superseded direct-control gameplay mutation now
/// owned by the canonical LAI action surface. `handle_client_text` rejects
/// those with the canonical update-required error after the bounded legacy
/// decode and before `apply_action`, so the retired lane can never reach the
/// authoritative world.
///
/// `false` is reserved for exactly the four bootstrap/lifecycle allowances the
/// canonical boundary documents: the Presence handshake, colony ensure, village
/// founding, and village selection. Accepting anything else here would restore
/// the second gameplay authority the cutover removes.
///
/// The match is exhaustive with no wildcard arm on purpose: a newly added
/// legacy variant must be classified explicitly instead of silently inheriting
/// an allowance.
fn legacy_action_requires_lai_v2(action: &ClientAction) -> bool {
    match action {
        // Bootstrap and village lifecycle — not gameplay mutation.
        ClientAction::Presence { .. }
        | ClientAction::Ensure
        | ClientAction::FoundVillage { .. }
        | ClientAction::JoinVillage { .. } => false,

        // Exact-tile construction, zone painting, and route authoring.
        ClientAction::PlanBuilding { .. }
        | ClientAction::CreateZone { .. }
        | ClientAction::RemoveZone { .. }
        | ClientAction::BuildRoad { .. }
        | ClientAction::BuildBridge { .. }
        | ClientAction::DesignateRail { .. }
        | ClientAction::BuildDock { .. }
        | ClientAction::BuildTransportVehicle { .. }
        | ClientAction::CreateTransportRoute { .. }
        | ClientAction::CancelTransportRoute { .. } => true,

        // Worker, officer, and labor assignment.
        ClientAction::AssignWorker { .. }
        | ClientAction::AssignOfficer { .. }
        | ClientAction::UnassignOfficer { .. }
        | ClientAction::SetCatLaborPreference { .. }
        | ClientAction::RequestJob { .. }
        | ClientAction::DispatchScout { .. } => true,

        // Production queues and station work slots.
        ClientAction::EditProductionQueue { .. }
        | ClientAction::EditProductionWorkSlot { .. } => true,

        // Player ballots and vote-kick.
        ClientAction::CastVote { .. } | ClientAction::RequestVoteKick { .. } => true,

        // Retired upgrade/research progression and direct boosts.
        ClientAction::PurchaseUpgrade { .. }
        | ClientAction::UnlockNode { .. }
        | ClientAction::ResearchNode { .. }
        | ClientAction::Boost { .. }
        | ClientAction::BoostCat { .. } => true,

        // Shrine tithe and offerings.
        ClientAction::OfferTithe { .. }
        | ClientAction::OfferMaterials { .. }
        | ClientAction::OfferResource { .. } => true,

        // Coin buy/sell and direct village trade.
        ClientAction::BuyResource { .. }
        | ClientAction::SellGoods { .. }
        | ClientAction::OfferVillageTrade { .. }
        | ClientAction::AcceptVillageTrade { .. }
        | ClientAction::CancelVillageTrade { .. } => true,

        // Farming, gathering, fishing, hauling, and storage designations.
        ClientAction::DesignateFarm { .. }
        | ClientAction::ClearFarm { .. }
        | ClientAction::DesignateStockpile { .. }
        | ClientAction::RemoveStockpile { .. }
        | ClientAction::DesignateGatherSpot { .. }
        | ClientAction::DesignateFishingSpot { .. }
        | ClientAction::RemoveGatherSpot { .. }
        | ClientAction::HaulGatherSpot { .. } => true,

        // Equipment, repair, and direct combat control.
        ClientAction::EquipItem { .. }
        | ClientAction::UnequipItem { .. }
        | ClientAction::RepairItem { .. }
        | ClientAction::TrainWarrior { .. }
        | ClientAction::DefendRaid { .. } => true,

        // Retired harness clock/seed controls: deterministic acceleration is a
        // canonical-lane concern, not a legacy shell allowance.
        ClientAction::SetTestAcceleration { .. }
        | ClientAction::AdvanceTime { .. }
        | ClientAction::SetTestRngSeed { .. } => true,
    }
}

async fn handle_client_text(
    state: &AppState,
    connection: &mut ConnectionContext,
    text: &str,
) -> ServerActionResult {
    if serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|value| value.get("actionSchemaVersion").cloned())
        .is_some()
    {
        return handle_canonical_action_text(state, connection, text).await;
    }
    if reject_before_action_decode(text).is_ok()
        || serde_json::from_str::<serde_json::Value>(text)
            .ok()
            .and_then(|value| value.get("protocolVersion").cloned())
            .is_some()
    {
        // The retired LAI.24/25 action family cannot be safely converted to
        // schema-v2: it exposed worker, route, stock, and appointment-shaped
        // mutations which the canonical protocol deliberately omits.
        return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
            canonical_update_required_error(),
        ));
    }
    let Ok(action) = serde_json::from_str::<ClientAction>(text) else {
        return ServerActionResult::fail("Invalid action.");
    };
    // The production retirement gate: every superseded gameplay mutation is
    // refused here, between the bounded legacy decode above and `apply_action`
    // below, so no retired direct control reaches the authoritative world.
    if legacy_action_requires_lai_v2(&action) {
        return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
            canonical_update_required_error(),
        ));
    }

    let now = now_ms();
    let peer_limiter_key = connection.peer_limiter_key();
    {
        let mut limiter = state.ip_rate_limiter.lock().await;
        limiter.prune(now);
        if !limiter.check(&peer_limiter_key, now) {
            return ServerActionResult::fail("Too many actions from this address — slow down.");
        }
    }
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
        let nickname = match &action {
            ClientAction::Presence { nickname, .. } => match normalized_player_name(nickname) {
                Ok(nickname) => nickname,
                Err(message) => return ServerActionResult::fail(message),
            },
            _ => unreachable!("presence authentication belongs to Presence"),
        };
        let peer = connection.peer_ip.map(|ip| ip.to_string());
        let valid_existing = verify_session_at(session_id, sig, state.session_secret.as_str(), now);
        let signed = if valid_existing {
            let signed = signed_session(session_id.to_owned(), state.session_secret.as_str());
            let mut guard = state.abuse_guard.lock().await;
            if let Some(peer) = peer {
                // Attribute founding to the current direct socket peer. This is an abuse
                // boundary, not an authentication factor: legitimate mobile users may move
                // networks while their signed bearer remains valid.
                guard.player_peers.insert(signed.player_id.clone(), peer);
            }
            signed
        } else if let Some(signed) =
            renew_session_at(session_id, sig, state.session_secret.as_str(), now)
        {
            let mut guard = state.abuse_guard.lock().await;
            if let Some(peer) = peer {
                guard.player_peers.insert(signed.player_id.clone(), peer);
            }
            signed
        } else {
            let mut guard = state.abuse_guard.lock().await;
            guard.session_issuance.prune(now);
            if !guard.session_issuance.check(&peer_limiter_key, now) {
                warn!(
                    peer = %peer_limiter_key,
                    max_new_sessions = SESSION_ISSUE_LIMIT_MAX,
                    window_ms = SESSION_ISSUE_LIMIT_WINDOW_MS,
                    "rejected new signed session issuance limit"
                );
                return ServerActionResult::fail(
                    "Too many new sessions from this address. Reuse the session already issued to this browser.",
                );
            }
            let signed = issue_session(state.session_secret.as_str(), now);
            if let Some(peer) = peer {
                guard.player_peers.insert(signed.player_id.clone(), peer);
            }
            signed
        };
        let identity_changed = connection
            .identity
            .as_ref()
            .is_some_and(|identity| identity.session_id != signed.session_id)
            || connection.identity.is_none();
        if identity_changed {
            let directory = state.village_directory.read().await;
            connection.colony_id = global_village_id(&directory);
            connection.nickname = None;
        }
        if let Some(nickname) = nickname {
            let db = state.db.lock().await;
            if let Err(err) =
                persistence::record_player_name(&db, &signed.player_id, &nickname, now)
            {
                error!(error = %err, "failed to persist player display name");
                return ServerActionResult::fail("Player name could not be saved.");
            }
            connection.nickname = Some(nickname);
        }
        let trusted = match canonical_trusted_session(&signed, now) {
            Ok(trusted) => trusted,
            Err(_) => return ServerActionResult::fail("Session identity is not valid."),
        };
        let session_row = lai65::CanonicalSessionRow::from_trusted(&trusted);
        let db = state.db.lock().await;
        if let Err(error) = leader_ai_persistence::save_canonical_session_row(&db, &session_row) {
            error!(%error, "failed to persist canonical session metadata");
            return ServerActionResult::fail("Session could not be saved.");
        }
        drop(db);
        connection.identity = Some(signed.clone());
        connection.leader_ai_protocol = true;
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

    // Older clients put their display name only on signed actions. Accept that
    // shape as a compatibility fallback, bind it to the authenticated player,
    // and persist it exactly like the newer named Presence handshake.
    let embedded_name = embedded_action_nickname(&action)
        .as_deref()
        .and_then(|name| normalized_player_name(name).ok().flatten());
    let actor_name = if matches!(authentication, ActionAuthentication::TestOnly) {
        None
    } else {
        connection.nickname.clone().or(embedded_name)
    };
    if connection.nickname.is_none()
        && let Some(nickname) = actor_name.as_deref()
    {
        let db = state.db.lock().await;
        if let Err(err) = persistence::record_player_name(&db, &identity.player_id, nickname, now) {
            error!(error = %err, "failed to persist player display name from signed action");
            return ServerActionResult::fail("Player name could not be saved.");
        }
        connection.nickname = Some(nickname.to_owned());
    }

    let ctx = ActionCtx {
        session_id: identity.session_id,
        player_id: identity.player_id,
        colony_id: connection.colony_id.clone(),
        now_ms: now,
    };

    let player_peers = if matches!(action, ClientAction::FoundVillage { .. }) {
        Some(state.abuse_guard.lock().await.player_peers.clone())
    } else {
        None
    };
    let mut world = state.world.lock().await;
    if matches!(action, ClientAction::FoundVillage { .. }) {
        if world.colonies.len() >= MAX_TOTAL_COLONIES {
            return ServerActionResult::fail("The shared world has reached its village capacity.");
        }
        if let Some(peer_ip) = connection.peer_ip {
            let peer = peer_ip.to_string();
            let player_peers = player_peers
                .as_ref()
                .expect("founding snapshots the peer directory");
            let personal_villages_from_peer = world
                .colonies
                .iter()
                .filter(|colony| colony.kind == VillageKind::Personal)
                .filter_map(|colony| colony.owner_player_id.as_ref())
                .filter(|owner| player_peers.get(*owner) == Some(&peer))
                .count();
            if personal_villages_from_peer >= MAX_PERSONAL_VILLAGES_PER_IP {
                return ServerActionResult::fail(
                    "This network address has reached its personal-village capacity.",
                );
            }
        }
    }
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
        if let (Some(actor_name), Some(message)) =
            (actor_name.as_deref(), action_audit_message(&action))
        {
            append_player_action_event(&mut world, &connection.colony_id, actor_name, message, now);
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

struct DirectoryOwnershipSource<'a> {
    directory: &'a BTreeMap<String, VillageDirectoryEntry>,
}

impl SelectedColonyOwnershipSource for DirectoryOwnershipSource<'_> {
    fn control_policy(&self, colony_id: &str) -> Option<ColonyControlPolicy> {
        self.directory.get(colony_id).map(|entry| match entry.kind {
            VillageKind::Global => ColonyControlPolicy::GlobalVillage,
            VillageKind::Personal => ColonyControlPolicy::PlayerOwned {
                owner_player_id: entry.owner_player_id.clone().unwrap_or_default(),
            },
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct LeaderAiMutationRateLimit;

async fn check_rate_limit_before_world_lock(
    state: &AppState,
    connection: &ConnectionContext,
    now: i64,
) -> Result<LeaderAiMutationRateLimit, ServerActionConflict> {
    let limiter_key = connection.limiter_key();
    let mut limiter = state.rate_limiter.lock().await;
    limiter.prune(now);
    if limiter.check(&limiter_key, now) {
        Ok(LeaderAiMutationRateLimit)
    } else {
        Err(ServerActionConflict::RateLimited {
            retry_after_ms: u64::try_from(ACTION_LIMIT_WINDOW_MS).unwrap_or_default(),
        })
    }
}

fn check_rate_limit_before_database_transaction(
    proof: LeaderAiMutationRateLimit,
) -> LeaderAiMutationRateLimit {
    proof
}

fn check_rate_limit_before_snapshot_build(
    proof: LeaderAiMutationRateLimit,
) -> LeaderAiMutationRateLimit {
    proof
}

struct AuthoritativeWorldWrite<'a>(&'a Mutex<WorldState>);

impl<'a> AuthoritativeWorldWrite<'a> {
    async fn write(&self) -> tokio::sync::MutexGuard<'a, WorldState> {
        self.0.lock().await
    }
}

/// The canonical LAI.64 route is deliberately a separate ingress path.  It
/// never decodes an action into the retired LAI.24/25 DTO and it binds the
/// payload identity to the already HMAC-verified socket session before it
/// reads or mutates a colony.
async fn handle_canonical_action_text(
    state: &AppState,
    connection: &mut ConnectionContext,
    text: &str,
) -> ServerActionResult {
    if lai65::is_canonical_test_reset_request(text) {
        return handle_canonical_test_reset_request(state, connection, text).await;
    }
    let now = now_ms();
    {
        let mut limiter = state.ip_rate_limiter.lock().await;
        limiter.prune(now);
        if !limiter.check(&connection.peer_limiter_key(), now) {
            return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                lai65::CanonicalBoundaryError::RateLimited {
                    retry_after_ms: u64::try_from(ACTION_LIMIT_WINDOW_MS).unwrap_or_default(),
                }
                .action_error(),
            ));
        }
    }
    let envelope = match CanonicalActionEnvelope::decode_json(text) {
        Ok(envelope) => envelope,
        Err(error) => {
            return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                lai65::CanonicalBoundaryError::Wire(error).action_error(),
            ));
        }
    };
    let Some(identity) = connection.identity.as_ref() else {
        return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
            lai65::CanonicalBoundaryError::Unauthenticated.action_error(),
        ));
    };
    if !verify_session_at(
        &identity.session_id,
        Some(&identity.sig),
        state.session_secret.as_str(),
        now,
    ) {
        return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
            lai65::CanonicalBoundaryError::Unauthenticated.action_error(),
        ));
    }
    {
        let mut limiter = state.rate_limiter.lock().await;
        limiter.prune(now);
        if !limiter.check(&connection.limiter_key(), now) {
            return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                lai65::CanonicalBoundaryError::RateLimited {
                    retry_after_ms: u64::try_from(ACTION_LIMIT_WINDOW_MS).unwrap_or_default(),
                }
                .action_error(),
            ));
        }
    }
    let trusted = match canonical_trusted_session(identity, now) {
        Ok(session) => session,
        Err(error) => {
            return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                error.action_error(),
            ));
        }
    };
    connection.leader_ai_protocol = true;
    if envelope.selected_colony_id.as_str() != connection.colony_id {
        // A websocket has one selected colony.  Base-game JoinVillage remains
        // the authenticated selection mechanism; canonical God actions never
        // smuggle a cross-colony selection switch.
        return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
            canonical_denied_error(),
        ));
    }

    let directory = state.village_directory.read().await;
    let directory_adapter = CanonicalDirectory {
        directory: &directory,
    };
    let authorized =
        match lai65::authorize_canonical_action(trusted, envelope, &directory_adapter, now) {
            Ok(action) => action,
            Err(error) => {
                return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                    error.action_error(),
                ));
            }
        };
    drop(directory);

    let mut replay = state.canonical_replay.lock().await;
    let mut hole_clicks = state.canonical_hole_clicks.lock().await;
    let mut reset_gate = state.canonical_test_reset.lock().await;
    let replay_before = replay.clone();
    let hole_clicks_before = hole_clicks.clone();
    let reset_gate_before = reset_gate.clone();
    let mut world = state.world.lock().await;
    let versions = CanonicalWorldVersions { world: &world };
    let build = if state.allow_test_actions {
        lai65::CanonicalServerBuild::TestBuild
    } else {
        lai65::CanonicalServerBuild::Production
    };
    let ingress = match lai65::admit_canonical_action(
        authorized,
        &versions,
        build,
        &mut *reset_gate,
        &replay,
        &mut hole_clicks,
        now,
    ) {
        Ok(ingress) => ingress,
        Err(error) => {
            return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                error.action_error(),
            ));
        }
    };
    let authorized = match ingress {
        lai65::CanonicalIngress::Replay(receipt) => {
            return ServerActionResult::from_canonical(CanonicalActionResponse::Receipt(receipt));
        }
        lai65::CanonicalIngress::Dispatch(action) => action,
    };
    if replay.len() >= lai65::MAX_CANONICAL_REPLAY_ROWS {
        *hole_clicks = hole_clicks_before;
        *reset_gate = reset_gate_before;
        return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
            lai65::CanonicalBoundaryError::ReplayStoreAtCapacity.action_error(),
        ));
    }

    let before = world.clone();
    if let Err(error) = apply_canonical_dispatch(&mut world, &authorized, now) {
        *world = before;
        *replay = replay_before;
        *hole_clicks = hole_clicks_before;
        *reset_gate = reset_gate_before;
        return ServerActionResult::from_canonical(CanonicalActionResponse::Error(error));
    }
    let receipt = canonical_receipt(&world, &authorized);
    let replay_row = match replay.record(&authorized, receipt.clone(), now) {
        Ok(row) => row,
        Err(_) => {
            *world = before;
            *replay = replay_before;
            *hole_clicks = hole_clicks_before;
            *reset_gate = reset_gate_before;
            return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                canonical_persistence_error(),
            ));
        }
    };
    let boundary_batch = lai65::CanonicalAtomicPersistenceBatch {
        replay_row,
        rate_rows: hole_clicks.rows(),
        session_row: lai65::CanonicalSessionRow::from_trusted(authorized.trusted_session()),
        consumed_reset_challenge: match authorized.dispatch() {
            lai65::CanonicalGodDispatch::SignedTestReset { nonce } => {
                Some(lai65::CanonicalResetChallengeKey {
                    session_id: authorized.trusted_session().session_id().clone(),
                    nonce: nonce.clone(),
                })
            }
            _ => None,
        },
    };
    let db = state.db.lock().await;
    if save_world_with_canonical_boundary(&db, &world, &boundary_batch).is_err() {
        *world = before;
        *replay = replay_before;
        *hole_clicks = hole_clicks_before;
        *reset_gate = reset_gate_before;
        return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
            canonical_persistence_error(),
        ));
    }
    drop(db);
    let refreshed_snapshot = build_snapshot(&world, now, state.online_count.load(Ordering::SeqCst));
    drop(world);
    *state.completed_snapshot.write().await = refreshed_snapshot.clone();
    let _ = state.snapshots.send(refreshed_snapshot);
    ServerActionResult::from_canonical(CanonicalActionResponse::Receipt(receipt))
}

/// Stage one of the test-only reset is deliberately separate from the
/// protocol's world-mutating `signed_test_reset` confirmation. It shares the
/// canonical header and all authorization checks, then persists only the
/// short-lived challenge. The confirmation consumes that exact durable row in
/// the same transaction as the selected-colony reset and receipt.
async fn handle_canonical_test_reset_request(
    state: &AppState,
    connection: &mut ConnectionContext,
    text: &str,
) -> ServerActionResult {
    let now = now_ms();
    {
        let mut limiter = state.ip_rate_limiter.lock().await;
        limiter.prune(now);
        if !limiter.check(&connection.peer_limiter_key(), now) {
            return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                lai65::CanonicalBoundaryError::RateLimited {
                    retry_after_ms: u64::try_from(ACTION_LIMIT_WINDOW_MS).unwrap_or_default(),
                }
                .action_error(),
            ));
        }
    }
    let request = match lai65::CanonicalTestResetRequest::decode_json(text) {
        Ok(request) => request,
        Err(error) => {
            return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                lai65::CanonicalBoundaryError::Wire(error).action_error(),
            ));
        }
    };
    let Some(identity) = connection.identity.as_ref() else {
        return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
            lai65::CanonicalBoundaryError::Unauthenticated.action_error(),
        ));
    };
    if !verify_session_at(
        &identity.session_id,
        Some(&identity.sig),
        state.session_secret.as_str(),
        now,
    ) {
        return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
            lai65::CanonicalBoundaryError::Unauthenticated.action_error(),
        ));
    }
    {
        let mut limiter = state.rate_limiter.lock().await;
        limiter.prune(now);
        if !limiter.check(&connection.limiter_key(), now) {
            return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                lai65::CanonicalBoundaryError::RateLimited {
                    retry_after_ms: u64::try_from(ACTION_LIMIT_WINDOW_MS).unwrap_or_default(),
                }
                .action_error(),
            ));
        }
    }
    let trusted = match canonical_trusted_session(identity, now) {
        Ok(session) => session,
        Err(error) => {
            return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                error.action_error(),
            ));
        }
    };
    if request.selected_colony_id.as_str() != connection.colony_id {
        return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
            canonical_denied_error(),
        ));
    }
    let directory = state.village_directory.read().await;
    let directory_adapter = CanonicalDirectory {
        directory: &directory,
    };
    let authorized = match lai65::authorize_canonical_test_reset_request(
        trusted,
        request,
        &directory_adapter,
        now,
    ) {
        Ok(request) => request,
        Err(error) => {
            return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                error.action_error(),
            ));
        }
    };
    drop(directory);
    if !state.allow_test_actions {
        return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
            lai65::CanonicalBoundaryError::SignedTestResetDisabled.action_error(),
        ));
    }
    let lai65::CanonicalTestResetRequestPayload::SignedTestResetRequest { nonce, signature } =
        &authorized.request().payload;
    let mut reset_gate = state.canonical_test_reset.lock().await;
    let before = reset_gate.clone();
    let verifier = ServerTestResetSignatureVerifier {
        session_secret: state.session_secret.as_str(),
    };
    let stage = match reset_gate.stage_first_step_for_colony(
        authorized.trusted_session(),
        authorized.request().selected_colony_id.clone(),
        authorized.request().idempotency_id.clone(),
        nonce.clone(),
        signature.clone(),
        now,
        &verifier,
    ) {
        Ok(stage) => stage,
        Err(error) => {
            return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                error.action_error(),
            ));
        }
    };
    if matches!(stage, lai65::TestResetStage::Staged) {
        let Some(challenge) =
            reset_gate.staged_challenge(authorized.trusted_session().session_id(), nonce)
        else {
            *reset_gate = before;
            return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                canonical_persistence_error(),
            ));
        };
        let (Some(selected_colony_id), Some(stage_idempotency_id)) =
            (challenge.selected_colony_id, challenge.stage_idempotency_id)
        else {
            *reset_gate = before;
            return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                canonical_persistence_error(),
            ));
        };
        let row = leader_ai_persistence::CanonicalResetChallengeRow {
            row_schema_version: lai65::CANONICAL_PERSISTENCE_ROW_SCHEMA_VERSION,
            session_id: challenge.session_id,
            authenticated_player_id: challenge.authenticated_player_id,
            selected_colony_id,
            stage_idempotency_id,
            nonce: challenge.nonce,
            signature: challenge.signature,
            expires_at_ms: challenge.expires_at_ms,
        };
        let db = state.db.lock().await;
        if leader_ai_persistence::save_canonical_test_reset_row(&db, &row).is_err() {
            *reset_gate = before;
            return ServerActionResult::from_canonical(CanonicalActionResponse::Error(
                canonical_persistence_error(),
            ));
        }
    }
    let receipt = ActionReceipt {
        idempotency_id: authorized.request().idempotency_id.clone(),
        selected_colony_id: authorized.request().selected_colony_id.clone(),
        outcome: if matches!(stage, lai65::TestResetStage::Staged) {
            ActionOutcome::Accepted
        } else {
            ActionOutcome::Replayed
        },
        changed_ids: vec![nonce.clone()],
        reason: Some(
            ReportText::new("Signed test-reset confirmation staged.".to_owned())
                .expect("constant staging receipt is valid"),
        ),
        committed_versions: Vec::new(),
    };
    ServerActionResult::from_canonical(CanonicalActionResponse::Receipt(receipt))
}

struct ServerTestResetSignatureVerifier<'a> {
    session_secret: &'a str,
}

impl lai65::TestResetSignatureVerifier for ServerTestResetSignatureVerifier<'_> {
    fn verify_first_step(
        &self,
        session: &lai65::TrustedCanonicalSession,
        nonce: &StableId,
        signature: &ReportText,
    ) -> bool {
        identity::session_signature_valid(
            &lai65::test_reset_signature_message(session, nonce),
            Some(signature.as_str()),
            self.session_secret,
        )
    }
}

struct CanonicalDirectory<'a> {
    directory: &'a BTreeMap<String, VillageDirectoryEntry>,
}

impl lai65::CanonicalColonyDirectory for CanonicalDirectory<'_> {
    fn selected_colony_access(&self, colony_id: &StableId) -> Option<lai65::CanonicalColonyAccess> {
        self.directory
            .get(colony_id.as_str())
            .and_then(|entry| match entry.kind {
                VillageKind::Global => Some(lai65::CanonicalColonyAccess::GlobalVillage),
                VillageKind::Personal => {
                    StableId::new(entry.owner_player_id.clone()?)
                        .ok()
                        .map(
                            |owner_player_id| lai65::CanonicalColonyAccess::PersonalVillage {
                                owner_player_id,
                            },
                        )
                }
            })
    }
}

struct CanonicalWorldVersions<'a> {
    world: &'a WorldState,
}

impl lai65::CanonicalVersionSource for CanonicalWorldVersions<'_> {
    fn current_version(&self, colony_id: &StableId, lane: VersionLane) -> Option<u64> {
        let runtime = &self
            .world
            .colonies
            .iter()
            .find(|colony| colony.id == colony_id.as_str())?
            .leader_ai_runtime;
        Some(canonical_lane_version(runtime, lane))
    }
}

fn canonical_trusted_session(
    identity: &SignedSession,
    now: i64,
) -> Result<lai65::TrustedCanonicalSession, lai65::CanonicalBoundaryError> {
    let session_id =
        StableId::new(identity.session_id.clone()).map_err(lai65::CanonicalBoundaryError::Wire)?;
    let player_id =
        StableId::new(identity.player_id.clone()).map_err(lai65::CanonicalBoundaryError::Wire)?;
    let expires_at_ms = session_issued_at(identity.session_id.as_str())
        .unwrap_or(now)
        .checked_add(SESSION_MAX_AGE_MS)
        .ok_or(lai65::CanonicalBoundaryError::InvalidTrustedSession)?;
    lai65::TrustedCanonicalSession::new(session_id, player_id, expires_at_ms)
}

fn canonical_lane_version(
    runtime: &cat_sim::leader_ai_runtime::LeaderAiRuntimeState,
    lane: VersionLane,
) -> u64 {
    leader_ai_snapshot_projection::canonical_lane_version(runtime, lane)
}

fn canonical_receipt(
    world: &WorldState,
    action: &lai65::AuthorizedCanonicalAction,
) -> ActionReceipt {
    let runtime = &world
        .colonies
        .iter()
        .find(|colony| colony.id == action.envelope().selected_colony_id.as_str())
        .expect("canonical authorization selects an existing colony")
        .leader_ai_runtime;
    let committed_versions = action
        .envelope()
        .payload
        .required_lanes()
        .iter()
        .copied()
        .map(|lane| VersionExpectation {
            lane,
            expected_version: canonical_lane_version(runtime, lane),
        })
        .collect();
    ActionReceipt {
        idempotency_id: action.envelope().idempotency_id.clone(),
        selected_colony_id: action.envelope().selected_colony_id.clone(),
        outcome: ActionOutcome::Accepted,
        changed_ids: vec![action.envelope().idempotency_id.clone()],
        reason: None,
        committed_versions,
    }
}

fn apply_canonical_dispatch(
    world: &mut WorldState,
    action: &lai65::AuthorizedCanonicalAction,
    now: i64,
) -> Result<(), ActionErrorSnapshot> {
    use cat_sim::{
        cat_governance::ExpulsionScope,
        construction_miracle_runtime::{ApplyConstructionMiracle, apply_construction_miracle},
        divine_action_offers::{
            DivineBoostOfferCatalog, EmergencyRescueWitnessSet, TrustedBoostActivation,
            TrustedEmergencyRescue,
        },
        divine_boosts::{DivineBoostActor, DivineBoostAuthorization},
        divine_hole_authority::{ClickBatchRequest, DivineHoleCommand, DivineHoleCommandEnvelope},
        food_divine_policy::{ConservationNudge, EmergencySupplyKind, HOLE_DELIVERY_APRON_SITE_ID},
        governance_authority::{BackingActor, BackingCommand, BackingEligibilityWire},
        moneyless_barter::PersonalStance as SimulationPersonalStance,
        physical_storage::StorageCompatibility,
        planner_core::PlannerId,
        player_directives::{BroadNudgeDirective, BroadNudgeDomain, BroadNudgeKey},
        progression_research::{PlayerPartitionKey, ProgressionCatalog, StudyId},
        research_authority::{ResearchCommand, ResearchCommandId, ResearchCommandKind},
        storage_authority::StorageAddress,
    };

    if matches!(
        action.dispatch(),
        lai65::CanonicalGodDispatch::SignedTestReset { .. }
    ) {
        return reset_selected_colony_for_test(
            world,
            action.envelope().selected_colony_id.as_str(),
            now,
        );
    }
    let world_colony_ids = world
        .colonies
        .iter()
        .map(|colony| colony.id.clone())
        .collect::<BTreeSet<_>>();
    let colony = world
        .colonies
        .iter_mut()
        .find(|colony| colony.id == action.envelope().selected_colony_id.as_str())
        .ok_or_else(canonical_denied_error)?;
    let runtime = &mut colony.leader_ai_runtime;
    let command_id = format!("canonical:{}", action.envelope().idempotency_id.as_str());
    let now_u64 = u64::try_from(now).map_err(|_| canonical_invalid_error())?;
    match action.dispatch() {
        lai65::CanonicalGodDispatch::ResearchQueue { study_id } => {
            let study_id = StudyId::new(study_id.as_str().to_owned())
                .map_err(|_| canonical_invalid_error())?;
            let catalog = ProgressionCatalog::from_embedded().map_err(|_| {
                canonical_adapter_error(
                    "action:research_catalog_unavailable",
                    "Research is not available right now.",
                )
            })?;
            runtime
                .research
                .apply(
                    &catalog,
                    ResearchCommand {
                        id: ResearchCommandId::derive(&runtime.research.colony_id, &command_id),
                        expected_version: runtime.research.version,
                        kind: ResearchCommandKind::QueueGodPath { target: study_id },
                    },
                )
                .map_err(|_| {
                    canonical_adapter_error(
                        "action:research_rejected",
                        "That research request cannot be applied.",
                    )
                })?;
            Ok(())
        }
        lai65::CanonicalGodDispatch::ResearchReorder {
            study_id,
            before_study_id,
        } => {
            let study_id = StudyId::new(study_id.as_str().to_owned())
                .map_err(|_| canonical_invalid_error())?;
            let before_study_id = before_study_id
                .as_ref()
                .map(|study_id| StudyId::new(study_id.as_str().to_owned()))
                .transpose()
                .map_err(|_| canonical_invalid_error())?;
            let catalog = ProgressionCatalog::from_embedded().map_err(|_| {
                canonical_adapter_error(
                    "action:research_catalog_unavailable",
                    "Research is not available right now.",
                )
            })?;
            runtime
                .research
                .apply(
                    &catalog,
                    ResearchCommand {
                        id: ResearchCommandId::derive(&runtime.research.colony_id, &command_id),
                        expected_version: runtime.research.version,
                        kind: ResearchCommandKind::ReorderGodTarget {
                            study_id,
                            before_study_id,
                        },
                    },
                )
                .map_err(|_| {
                    canonical_adapter_error(
                        "action:research_rejected",
                        "That research request cannot be applied.",
                    )
                })?;
            Ok(())
        }
        lai65::CanonicalGodDispatch::ResearchFund { study_id } => {
            let catalog = ProgressionCatalog::from_embedded().map_err(|_| {
                canonical_adapter_error(
                    "action:research_catalog_unavailable",
                    "Research is not available right now.",
                )
            })?;
            if runtime
                .research
                .god_front()
                .is_none_or(|front| front.study_id.as_str() != study_id.as_str())
            {
                return Err(canonical_adapter_error(
                    "action:research_front_changed",
                    "That research is no longer at the front of the God queue.",
                ));
            }
            runtime
                .research
                .apply(
                    &catalog,
                    ResearchCommand {
                        id: ResearchCommandId::derive(&runtime.research.colony_id, &command_id),
                        expected_version: runtime.research.version,
                        kind: ResearchCommandKind::FundGodFront {
                            consume_preparation: false,
                        },
                    },
                )
                .map_err(|_| {
                    canonical_adapter_error(
                        "action:research_rejected",
                        "That research request cannot be applied.",
                    )
                })?;
            Ok(())
        }
        lai65::CanonicalGodDispatch::ResearchRemove { study_id } => {
            let study_id = StudyId::new(study_id.as_str().to_owned())
                .map_err(|_| canonical_invalid_error())?;
            let catalog = ProgressionCatalog::from_embedded().map_err(|_| {
                canonical_adapter_error(
                    "action:research_catalog_unavailable",
                    "Research is not available right now.",
                )
            })?;
            runtime
                .research
                .apply(
                    &catalog,
                    ResearchCommand {
                        id: ResearchCommandId::derive(&runtime.research.colony_id, &command_id),
                        expected_version: runtime.research.version,
                        kind: ResearchCommandKind::RemoveGodTarget { study_id },
                    },
                )
                .map_err(|_| {
                    canonical_adapter_error(
                        "action:research_rejected",
                        "That research request cannot be applied.",
                    )
                })?;
            Ok(())
        }
        lai65::CanonicalGodDispatch::ResearchPreparation { study_id } => {
            let study_id = StudyId::new(study_id.as_str().to_owned())
                .map_err(|_| canonical_invalid_error())?;
            let catalog = ProgressionCatalog::from_embedded().map_err(|_| {
                canonical_adapter_error(
                    "action:research_catalog_unavailable",
                    "Research is not available right now.",
                )
            })?;
            runtime
                .research
                .apply(
                    &catalog,
                    ResearchCommand {
                        id: ResearchCommandId::derive(&runtime.research.colony_id, &command_id),
                        expected_version: runtime.research.version,
                        kind: ResearchCommandKind::RequestPreparation { study_id },
                    },
                )
                .map_err(|_| {
                    canonical_adapter_error(
                        "action:research_preparation_rejected",
                        "That physical research preparation cannot be started.",
                    )
                })?;
            Ok(())
        }
        lai65::CanonicalGodDispatch::FoodConservation { nudge_basis_points } => {
            let nudge = if *nudge_basis_points > 0 {
                ConservationNudge::FavorImmediateSurvival
            } else {
                ConservationNudge::ProtectScarceFood
            };
            runtime
                .divine_hole
                .apply(
                    DivineHoleCommandEnvelope::new(
                        command_id,
                        runtime.divine_hole.version,
                        DivineHoleCommand::SetConservationNudge { nudge },
                    )
                    .map_err(|_| canonical_invalid_error())?,
                )
                .map_err(|_| {
                    canonical_adapter_error(
                        "action:food_policy_rejected",
                        "The food policy could not be adjusted.",
                    )
                })?;
            Ok(())
        }
        lai65::CanonicalGodDispatch::HoleClickBatch {
            target_id,
            accepted_clicks,
            ..
        } => {
            runtime
                .divine_hole
                .apply(
                    DivineHoleCommandEnvelope::new(
                        command_id,
                        runtime.divine_hole.version,
                        DivineHoleCommand::AcceptClickBatch {
                            batch: ClickBatchRequest {
                                target_id: target_id.as_str().to_owned(),
                                player_id: action
                                    .trusted_session()
                                    .authenticated_player_id()
                                    .as_str()
                                    .to_owned(),
                                requested_clicks: *accepted_clicks,
                                client_batch_window_ms:
                                    cat_protocol::lai64::CANONICAL_CLICK_BATCH_WINDOW_MS,
                                now_real_ms: now_u64,
                            },
                        },
                    )
                    .map_err(|_| canonical_invalid_error())?,
                )
                .map_err(|_| {
                    canonical_adapter_error(
                        "action:hole_click_rejected",
                        "That Hole contribution is no longer available.",
                    )
                })?;
            Ok(())
        }
        lai65::CanonicalGodDispatch::Inspiration => {
            runtime
                .divine_hole
                .apply(
                    DivineHoleCommandEnvelope::new(
                        command_id,
                        runtime.divine_hole.version,
                        DivineHoleCommand::ActivateInspiration {
                            player_id: action
                                .trusted_session()
                                .authenticated_player_id()
                                .as_str()
                                .to_owned(),
                            now_real_ms: now_u64,
                        },
                    )
                    .map_err(|_| canonical_invalid_error())?,
                )
                .map_err(|_| {
                    canonical_adapter_error(
                        "action:inspiration_unavailable",
                        "Inspiration is not available yet.",
                    )
                })?;
            Ok(())
        }
        lai65::CanonicalGodDispatch::ActivateBoost { boost_id } => {
            let player_id = PlannerId::derive(
                "player",
                [action.trusted_session().authenticated_player_id().as_str()],
            );
            let partition = PlayerPartitionKey {
                colony_id: runtime.colony_partition.clone(),
                player_id: player_id.clone(),
            };
            let catalog = DivineBoostOfferCatalog::capture(
                partition,
                &runtime.divine_hole,
                &runtime.boosts,
                &runtime.research,
            )
            .map_err(|_| {
                canonical_adapter_error(
                    "action:boost_offer_unavailable",
                    "That Divine Boost offer is no longer available.",
                )
            })?;
            catalog
                .purchase_by_id(
                    boost_id.as_str(),
                    &runtime.divine_hole,
                    &mut runtime.boosts,
                    &mut runtime.research,
                    TrustedBoostActivation {
                        authorization: DivineBoostAuthorization {
                            actor: DivineBoostActor::Player {
                                player_id: player_id.clone(),
                            },
                            authenticated_player_id: Some(player_id),
                            owns_colony: true,
                        },
                        activated_tick: runtime.last_processed_tick.unwrap_or_default(),
                        ticks_per_game_hour: 60,
                    },
                )
                .map_err(|_| {
                    canonical_adapter_error(
                        "action:boost_activation_rejected",
                        "That Divine Boost could not be activated from the current offer.",
                    )
                })?;
            Ok(())
        }
        lai65::CanonicalGodDispatch::ConstructionMiracle { offer_id } => {
            let authenticated_player_id =
                action.trusted_session().authenticated_player_id().as_str();
            let offers = leader_ai_snapshot_projection::capture_construction_miracle_offers(
                runtime,
                authenticated_player_id,
            )
            .map_err(|_| {
                canonical_adapter_error(
                    "action:construction_miracle_offer_unavailable",
                    "That construction miracle offer is no longer available.",
                )
            })?;
            let offer = offers
                .iter()
                .find(|offer| offer.offer_id == offer_id.as_str())
                .ok_or_else(|| {
                    canonical_adapter_error(
                        "action:construction_miracle_offer_stale",
                        "The selected construction miracle does not match the current project report.",
                    )
                })?;
            let project_id = offer.project_id.clone();
            let player_id = PlannerId::derive("player", [authenticated_player_id]);
            let expected_authority_version = runtime.divine_hole.version;
            let expected_void_version = runtime.research.void.version;
            apply_construction_miracle(
                runtime,
                ApplyConstructionMiracle {
                    command_id,
                    project_id,
                    player_id: player_id.as_str().to_owned(),
                    expected_authority_version,
                    expected_void_version,
                    now_real_ms: now_u64,
                },
            )
            .map_err(|_| {
                canonical_adapter_error(
                    "action:construction_miracle_rejected",
                    "That construction miracle could not be applied to the current project bill.",
                )
            })?;
            Ok(())
        }
        lai65::CanonicalGodDispatch::EmergencyRescue { witness_id } => {
            let report_version = runtime.resident_needs_report_version.ok_or_else(|| {
                canonical_adapter_error(
                    "action:rescue_report_unavailable",
                    "Emergency rescue requires a current resident-needs report.",
                )
            })?;
            let summary = runtime.resident_needs_summary.ok_or_else(|| {
                canonical_adapter_error(
                    "action:rescue_report_unavailable",
                    "Emergency rescue requires a current resident-needs report.",
                )
            })?;
            let partition = PlayerPartitionKey {
                colony_id: runtime.colony_partition.clone(),
                player_id: PlannerId::derive(
                    "player",
                    [action.trusted_session().authenticated_player_id().as_str()],
                ),
            };
            let witnesses = EmergencyRescueWitnessSet::capture(
                partition,
                report_version,
                summary,
                &runtime.divine_hole,
                &runtime.research.void,
            )
            .map_err(|_| {
                canonical_adapter_error(
                    "action:rescue_report_stale",
                    "Emergency rescue evidence is no longer current.",
                )
            })?;
            let witness = witnesses
                .witnesses
                .iter()
                .find(|witness| witness.id.as_str() == witness_id.as_str())
                .ok_or_else(|| {
                    canonical_adapter_error(
                        "action:rescue_not_reported",
                        "The current village report does not authorize that emergency rescue.",
                    )
                })?;
            let supply = witness.supply;
            let envelope = witnesses
                .resolve_rescue(
                    &witness.id,
                    report_version,
                    summary,
                    &runtime.divine_hole,
                    &runtime.research.void,
                    TrustedEmergencyRescue {
                        command_id,
                        now_real_ms: now_u64,
                    },
                )
                .map_err(|_| {
                    canonical_adapter_error(
                        "action:rescue_rejected",
                        "That emergency rescue could not be applied.",
                    )
                })?;
            runtime
                .apply_void_action_and_materialize(
                    envelope,
                    StorageAddress::PurposeCargo {
                        site_id: HOLE_DELIVERY_APRON_SITE_ID.to_owned(),
                    },
                    match supply {
                        EmergencySupplyKind::DivineRation => StorageCompatibility::Food,
                        EmergencySupplyKind::DivineWater => StorageCompatibility::Liquid,
                    },
                )
                .map_err(|_| {
                    canonical_adapter_error(
                        "action:rescue_materialization_failed",
                        "The emergency supply could not be placed on the Hole delivery apron.",
                    )
                })?;
            Ok(())
        }
        lai65::CanonicalGodDispatch::CandidateBacking {
            election_id,
            candidate_id,
        } => {
            let global_village = colony.kind == VillageKind::Global;
            runtime
                .governance
                .submit_backing(BackingCommand {
                    idempotency_id: command_id,
                    expected_version: runtime.governance.version(),
                    election_id: election_id.as_str().to_owned(),
                    player_id: action
                        .trusted_session()
                        .authenticated_player_id()
                        .as_str()
                        .to_owned(),
                    candidate_cat_id: candidate_id.as_str().to_owned(),
                    actor: BackingActor {
                        eligibility: BackingEligibilityWire {
                            authenticated: true,
                            eligible_global_player: global_village,
                            personal_village_owner: !global_village,
                        },
                        global_village,
                    },
                    submitted_tick: runtime.last_processed_tick.unwrap_or_default(),
                })
                .map_err(|_| {
                    canonical_adapter_error(
                        "action:candidate_backing_rejected",
                        "That election backing cannot be applied.",
                    )
                })?;
            Ok(())
        }
        lai65::CanonicalGodDispatch::PersonalStance {
            other_colony_id,
            stance,
        } => {
            if !world_colony_ids.contains(other_colony_id.as_str()) {
                return Err(canonical_adapter_error(
                    "action:stance_target_unavailable",
                    "That village is not available for a personal stance.",
                ));
            }
            let from = cat_sim::diplomacy::DiplomacyColonyId::derive(&colony.id);
            let to = cat_sim::diplomacy::DiplomacyColonyId::derive(other_colony_id.as_str());
            let stance = match stance {
                cat_protocol::lai64::PersonalStance::Alliance => SimulationPersonalStance::Alliance,
                cat_protocol::lai64::PersonalStance::Neutral => SimulationPersonalStance::Neutral,
                cat_protocol::lai64::PersonalStance::Enemy => SimulationPersonalStance::Enemy,
            };
            runtime
                .trade
                .set_stance(
                    command_id,
                    format!(
                        "canonical_stance_v1:{}:{}:{stance:?}",
                        colony.id,
                        other_colony_id.as_str()
                    ),
                    runtime.trade.version(),
                    from,
                    to,
                    stance,
                )
                .map_err(|_| {
                    canonical_adapter_error(
                        "action:personal_stance_rejected",
                        "That personal stance cannot be applied.",
                    )
                })?;
            Ok(())
        }
        lai65::CanonicalGodDispatch::Expel {
            subject_cat_id,
            household,
        } => {
            runtime
                .governance
                .preview_expulsion(
                    &command_id,
                    subject_cat_id.as_str(),
                    if *household {
                        ExpulsionScope::WholeHousehold
                    } else {
                        ExpulsionScope::SelectedAdult
                    },
                )
                .map_err(|_| {
                    canonical_adapter_error(
                        "action:expulsion_rejected",
                        "That expulsion cannot enter the cleanup workflow.",
                    )
                })?;
            Ok(())
        }
        lai65::CanonicalGodDispatch::BroadDomainNudge {
            domain,
            building_kind_id,
            basis_points,
        } => {
            let domain = match domain {
                cat_protocol::lai64::NudgeDomain::Survival => BroadNudgeDomain::Survival,
                cat_protocol::lai64::NudgeDomain::Defense => BroadNudgeDomain::Defense,
                cat_protocol::lai64::NudgeDomain::Hole => BroadNudgeDomain::Hole,
                cat_protocol::lai64::NudgeDomain::Hunting => BroadNudgeDomain::Hunting,
                cat_protocol::lai64::NudgeDomain::Food => BroadNudgeDomain::Food,
                cat_protocol::lai64::NudgeDomain::Housing => BroadNudgeDomain::Housing,
                cat_protocol::lai64::NudgeDomain::Construction => BroadNudgeDomain::Construction,
                cat_protocol::lai64::NudgeDomain::Storage => BroadNudgeDomain::Storage,
                cat_protocol::lai64::NudgeDomain::Research => BroadNudgeDomain::Research,
                cat_protocol::lai64::NudgeDomain::Trade => BroadNudgeDomain::Trade,
                cat_protocol::lai64::NudgeDomain::Infrastructure => {
                    BroadNudgeDomain::Infrastructure
                }
            };
            runtime
                .player_directives
                .set_broad_nudge(BroadNudgeDirective {
                    key: BroadNudgeKey {
                        domain,
                        building_kind_id: building_kind_id
                            .as_ref()
                            .map(|id| id.as_str().to_owned()),
                    },
                    basis_points: *basis_points,
                    planning_epoch: runtime.planner.planning_epoch,
                })
                .map_err(|_| {
                    canonical_adapter_error(
                        "action:broad_nudge_rejected",
                        "That broad priority could not be applied to the current planning review.",
                    )
                })?;
            Ok(())
        }
        lai65::CanonicalGodDispatch::SignedTestReset { .. } => {
            unreachable!("signed test reset returns before borrowing the selected colony")
        }
    }
}

/// Recreate exactly one selected colony from its deterministic founding
/// fixture. The canonical route has already authenticated the signed session,
/// verified selected-colony ownership, and consumed a matching staged reset.
/// No neighboring colony is removed, recreated, or otherwise mutated.
fn reset_selected_colony_for_test(
    world: &mut WorldState,
    selected_colony_id: &str,
    now: i64,
) -> Result<(), ActionErrorSnapshot> {
    use cat_sim::world_tick::{found_colony_at, found_global_colony, publish_colony_spatial};

    let colony_index = world
        .colonies
        .iter()
        .position(|colony| colony.id == selected_colony_id)
        .ok_or_else(canonical_denied_error)?;
    let current = &world.colonies[colony_index];
    let colony_id = current.id.clone();
    let colony_name = current.name.clone();
    let kind = current.kind;
    let scale = current.scale;
    let owner_player_id = current.owner_player_id.clone();
    let anchor = current.anchor;
    let seed = current.test_rng_seed.unwrap_or(world.world_seed);
    let mut reset = if kind == VillageKind::Global {
        found_global_colony(world.world_seed, colony_id, now, seed)
    } else {
        found_colony_at(world.world_seed, colony_id, now, seed, anchor)
    };
    reset.name = colony_name;
    reset.kind = kind;
    reset.scale = scale;
    reset.owner_player_id = owner_player_id;
    world.colonies[colony_index] = reset;
    publish_colony_spatial(&mut world.shared_spatial, &world.colonies[colony_index]);
    Ok(())
}

fn canonical_adapter_error(code: &str, reason: &str) -> ActionErrorSnapshot {
    ActionErrorSnapshot {
        code: StableId::new(code.to_owned()).expect("constant canonical action error code"),
        reason: cat_protocol::lai64::ReportText::new(reason.to_owned())
            .expect("constant canonical action error reason"),
        retry_after_ms: None,
        refresh_versions: Vec::new(),
    }
}

fn canonical_invalid_error() -> ActionErrorSnapshot {
    lai65::CanonicalBoundaryError::Wire(cat_protocol::lai64::CanonicalWireError::MalformedPayload)
        .action_error()
}

fn canonical_denied_error() -> ActionErrorSnapshot {
    lai65::CanonicalBoundaryError::SelectedColonyDenied.action_error()
}

fn canonical_persistence_error() -> ActionErrorSnapshot {
    lai65::CanonicalBoundaryError::PersistenceCodec.action_error()
}

fn canonical_update_required_error() -> ActionErrorSnapshot {
    canonical_adapter_error(
        "action:update_required",
        "This server requires the canonical world-tick update for that action.",
    )
}

#[cfg(any())]
async fn legacy_lai25_client_text(
    state: &AppState,
    connection: &mut ConnectionContext,
    text: &str,
) -> ServerActionResult {
    let frame = match check_protocol_compatibility(text) {
        Ok(frame) => frame,
        Err(ServerActionConflict::UpdateRequired(response)) => {
            debug_assert_eq!(
                response.minimum_supported_version,
                minimum_supported_action_protocol_version()
            );
            debug_assert_eq!(
                response.current_protocol_version,
                current_action_protocol_version()
            );
            return ServerActionResult::from_leader_ai(LeaderAiServerActionResult::UpdateRequired(
                response,
            ));
        }
        Err(conflict) => {
            return ServerActionResult::from_leader_ai(LeaderAiServerActionResult::ProtocolError(
                Box::new(conflict.to_protocol_conflict()),
            ));
        }
    };
    let envelope = match decode_lai_action_envelope(frame) {
        Ok(envelope) => envelope,
        Err(conflict) => {
            return malformed_leader_ai_result(text, conflict);
        }
    };
    let Some(session) = connection.identity.as_ref() else {
        return rejected_leader_ai_result(&envelope, ServerActionConflict::Unauthenticated);
    };
    let now = now_ms();
    let verified = match check_hmac_session_authentication(
        session,
        state.session_secret.as_str(),
        now,
        &envelope,
    ) {
        Ok(verified) => verified,
        Err(conflict) => return rejected_leader_ai_result(&envelope, conflict),
    };
    connection.leader_ai_protocol = true;
    let founding_peer_directory = matches!(
        envelope.payload,
        cat_protocol::LeaderAiActionPayload::FoundVillage { .. }
    )
    .then(|| async { state.abuse_guard.lock().await.player_peers.clone() });
    let founding_peer_directory = match founding_peer_directory {
        Some(directory) => Some(directory.await),
        None => None,
    };
    let rate_limit = match check_rate_limit_before_world_lock(state, connection, now).await {
        Ok(proof) => proof,
        Err(conflict) => return rejected_leader_ai_result(&envelope, conflict),
    };
    let directory = state.village_directory.read().await;
    let ownership_source = DirectoryOwnershipSource {
        directory: &directory,
    };
    let ownership: SelectedColonyOwnershipGuard =
        match check_selected_colony_ownership(&ownership_source, &verified, &envelope) {
            Ok(ownership) => ownership,
            Err(conflict) => return rejected_leader_ai_result(&envelope, conflict),
        };
    let ownership: OwnsSelectedColony = ownership;
    drop(directory);
    if let Err(conflict) = check_actor_action_authority(
        ServerMutationActor::AuthenticatedPlayer(&verified),
        &envelope,
    ) {
        return rejected_leader_ai_result(&envelope, conflict);
    }
    let authorized = leader_ai_action_routing::AuthorizedMutation::new(
        envelope.clone(),
        verified.clone(),
        ownership,
    );

    let mut receipts = state.leader_ai_receipts.lock().await;
    let world = AuthoritativeWorldWrite(&state.world);
    let mut world = world.write().await;
    if matches!(
        envelope.payload,
        cat_protocol::LeaderAiActionPayload::FoundVillage { .. }
    ) {
        if world.colonies.len() >= MAX_TOTAL_COLONIES {
            return rejected_leader_ai_result(
                &envelope,
                ServerActionConflict::PreconditionFailed(report_safe("village_capacity")),
            );
        }
        if let Some(peer_ip) = connection.peer_ip {
            let peer = peer_ip.to_string();
            let personal_villages_from_peer = world
                .colonies
                .iter()
                .filter(|colony| colony.kind == VillageKind::Personal)
                .filter_map(|colony| colony.owner_player_id.as_ref())
                .filter(|owner| {
                    founding_peer_directory
                        .as_ref()
                        .and_then(|directory| directory.get(*owner))
                        == Some(&peer)
                })
                .count();
            if personal_villages_from_peer >= MAX_PERSONAL_VILLAGES_PER_IP {
                return rejected_leader_ai_result(
                    &envelope,
                    ServerActionConflict::PreconditionFailed(report_safe(
                        "network_village_capacity",
                    )),
                );
            }
        }
    }
    let before = world.clone();
    let receipts_before = receipts.clone();
    let mut executor = LiveLeaderAiMutationExecutor {
        world: &mut world,
        receipts: &mut receipts,
        now_ms: now,
    };
    let response =
        match LeaderAiServerMutationPipeline::execute_remaining(&authorized, &mut executor) {
            Ok(response) => response,
            Err(conflict) => {
                let projected = project_server_action_response(&envelope, &conflict);
                if is_receiptable_conflict(&conflict)
                    && let LeaderAiServerActionResult::Action(response) = &projected
                {
                    let response = response.as_ref().clone();
                    if receipts.record(&envelope, response.clone()).is_err()
                        || persist_runtime_action_receipt(&mut world, &envelope, &response, now)
                            .is_err()
                    {
                        *world = before;
                        *receipts = receipts_before;
                        return rejected_leader_ai_result(
                            &envelope,
                            ServerActionConflict::MalformedActionId,
                        );
                    }
                    let _database_rate_limit =
                        check_rate_limit_before_database_transaction(rate_limit);
                    let db = state.db.lock().await;
                    if save_world(&db, &world).is_err() {
                        *world = before;
                        *receipts = receipts_before;
                        return rejected_leader_ai_result(
                            &envelope,
                            ServerActionConflict::PreconditionFailed(report_safe(
                                "persistence_failed",
                            )),
                        );
                    }
                }
                return ServerActionResult::from_leader_ai(projected);
            }
        };
    if matches!(
        response.result,
        LeaderAiActionResult::DuplicateReplay { .. }
    ) {
        return ServerActionResult::from_leader_ai(LeaderAiServerActionResult::Action(Box::new(
            response,
        )));
    }
    if receipts.record(&envelope, response.clone()).is_err()
        || persist_runtime_action_receipt(&mut world, &envelope, &response, now).is_err()
    {
        *world = before;
        *receipts = receipts_before;
        return rejected_leader_ai_result(&envelope, ServerActionConflict::MalformedActionId);
    }
    let database_rate_limit = check_rate_limit_before_database_transaction(rate_limit);
    let db = state.db.lock().await;
    if save_world(&db, &world).is_err() {
        *world = before;
        *receipts = receipts_before;
        return rejected_leader_ai_result(
            &envelope,
            ServerActionConflict::PreconditionFailed(report_safe("persistence_failed")),
        );
    }
    drop(db);
    let selected_after = match &envelope.payload {
        cat_protocol::LeaderAiActionPayload::SelectColony { target_colony_id } => {
            Some(target_colony_id.as_str().to_owned())
        }
        cat_protocol::LeaderAiActionPayload::FoundVillage { .. } => world
            .colonies
            .iter()
            .find(|colony| {
                colony.kind == VillageKind::Personal
                    && colony.owner_player_id.as_deref() == Some(verified.player_id().as_str())
            })
            .map(|colony| colony.id.clone()),
        _ => None,
    };
    let refreshed_directory = matches!(
        envelope.payload,
        cat_protocol::LeaderAiActionPayload::FoundVillage { .. }
    )
    .then(|| village_directory(&world));
    let refreshed_snapshot = build_snapshot(&world, now, state.online_count.load(Ordering::SeqCst));
    drop(world);
    if let Some(directory) = refreshed_directory {
        *state.village_directory.write().await = directory;
    }
    if let Some(selected_after) = selected_after {
        connection.colony_id = selected_after;
    }
    *state.completed_snapshot.write().await = refreshed_snapshot.clone();
    let _ = state.snapshots.send(refreshed_snapshot);
    let _snapshot_rate_limit = check_rate_limit_before_snapshot_build(database_rate_limit);
    ServerActionResult::from_leader_ai(LeaderAiServerActionResult::Action(Box::new(response)))
}

#[cfg(any())]
fn malformed_leader_ai_result(text: &str, conflict: ServerActionConflict) -> ServerActionResult {
    let value = serde_json::from_str::<serde_json::Value>(text).ok();
    let envelope = value.and_then(|value| {
        serde_json::from_value::<LeaderAiActionEnvelope>(value)
            .ok()
            .filter(|envelope| !envelope.colony_id.as_str().is_empty())
    });
    if let Some(envelope) = envelope {
        rejected_leader_ai_result(&envelope, conflict)
    } else {
        ServerActionResult::from_leader_ai(LeaderAiServerActionResult::ProtocolError(Box::new(
            conflict.to_protocol_conflict(),
        )))
    }
}

#[cfg(any())]
fn rejected_leader_ai_result(
    envelope: &LeaderAiActionEnvelope,
    conflict: ServerActionConflict,
) -> ServerActionResult {
    ServerActionResult::from_leader_ai(project_server_action_response(envelope, &conflict))
}

#[cfg(any())]
fn is_receiptable_conflict(conflict: &ServerActionConflict) -> bool {
    matches!(
        conflict,
        ServerActionConflict::PreconditionFailed(_)
            | ServerActionConflict::InsufficientFavor(_)
            | ServerActionConflict::ReservationConflict(_)
    )
}

#[cfg(any())]
fn persist_runtime_action_receipt(
    world: &mut WorldState,
    envelope: &LeaderAiActionEnvelope,
    response: &LeaderAiActionResponse,
    now_ms: i64,
) -> Result<(), ServerActionConflict> {
    let colony = selected_colony_mut(world, envelope)?;
    let planner_colony_id =
        cat_sim::planner_core::PlannerId::derive("colony", [envelope.colony_id.as_str()]);
    let action_key = format!(
        "{}:{}",
        envelope.player_id.as_str(),
        envelope.idempotency_id.as_str()
    );
    let id = cat_sim::leader_ai_runtime::RuntimeMutationId::derive(
        "lai27_action",
        &planner_colony_id,
        &action_key,
    );
    let request_fingerprint =
        serde_json::to_string(envelope).map_err(|_| ServerActionConflict::MalformedPayload)?;
    let response_json =
        serde_json::to_string(response).map_err(|_| ServerActionConflict::MalformedPayload)?;
    let committed_tick = tick_from_ms(now_ms);
    let expires_tick = committed_tick.saturating_add(7 * 24 * 60 * 60 * 1_000);
    let receipts = &mut colony.leader_ai_runtime.idempotency_receipts;
    if let Some(existing) = receipts.get(&id) {
        return if existing.request_fingerprint == request_fingerprint
            && existing.response_json == response_json
        {
            Ok(())
        } else {
            Err(ServerActionConflict::MalformedActionId)
        };
    }
    if receipts.len() == cat_sim::leader_ai_runtime::MAX_RUNTIME_IDEMPOTENCY_RECEIPTS
        && let Some(oldest) = receipts
            .iter()
            .min_by_key(|(_, receipt)| (receipt.expires_tick, receipt.committed_tick))
            .map(|(id, _)| id.clone())
    {
        receipts.remove(&oldest);
    }
    receipts.insert(
        id.clone(),
        cat_sim::leader_ai_runtime::RuntimeIdempotencyReceipt {
            id,
            committed_tick,
            expires_tick,
            request_fingerprint,
            response_json,
        },
    );
    colony
        .leader_ai_runtime
        .validate()
        .map_err(|_| ServerActionConflict::MalformedPayload)
}

#[cfg(any())]
struct LiveLeaderAiMutationExecutor<'a> {
    world: &'a mut WorldState,
    receipts: &'a mut IdempotencyReceiptStore,
    now_ms: i64,
}

#[cfg(any())]
impl OrderedMutationExecutor for LiveLeaderAiMutationExecutor<'_> {
    fn check_expected_state_versions(
        &mut self,
        authorized: &leader_ai_action_routing::AuthorizedMutation,
        expected: leader_ai_action_routing::ExpectedServerStateVersions<'_>,
    ) -> Result<(), ServerActionConflict> {
        if !matches!(
            self.receipts
                .check_bounded_idempotent_replay(authorized.envelope())?,
            IdempotencyReplay::Missing
        ) {
            return Ok(());
        }
        let current =
            current_server_state_versions(self.world, authorized.envelope().colony_id.as_str())
                .ok_or(ServerActionConflict::OpaqueExistenceDenied)?;
        check_expected_state_versions(expected.expected(), &current)
    }

    fn check_bounded_idempotent_replay(
        &mut self,
        authorized: &leader_ai_action_routing::AuthorizedMutation,
    ) -> Result<Option<LeaderAiActionResponse>, ServerActionConflict> {
        match self
            .receipts
            .check_bounded_idempotent_replay(authorized.envelope())?
        {
            IdempotencyReplay::Missing => Ok(None),
            IdempotencyReplay::ReplayAcceptedPriorResult(response)
            | IdempotencyReplay::ReplayRejectedPriorResult(response) => Ok(Some(response)),
        }
    }

    fn check_current_preconditions(
        &mut self,
        authorized: &leader_ai_action_routing::AuthorizedMutation,
    ) -> Result<(), ServerActionConflict> {
        let _no_mutation = NoMutationBeforePreconditions;
        let mut candidate = self.world.clone();
        apply_live_leader_ai_action(
            &mut candidate,
            authorized.envelope(),
            authorized.verified_session(),
            self.now_ms,
        )
    }

    fn commit_atomic_favor_reservation_state(
        &mut self,
        authorized: &leader_ai_action_routing::AuthorizedMutation,
    ) -> Result<LeaderAiActionResponse, ServerActionConflict> {
        let envelope = authorized.envelope();
        let mut transaction = AtomicLeaderAiCommit::stage(self.world);
        apply_live_leader_ai_action(
            transaction.candidate_mut(),
            envelope,
            authorized.verified_session(),
            self.now_ms,
        )?;
        let committed_versions =
            current_server_state_versions(transaction.candidate_mut(), envelope.colony_id.as_str())
                .ok_or(ServerActionConflict::OpaqueExistenceDenied)?;
        transaction.commit_favor_debit_once(self.world);
        Ok(LeaderAiActionResponse {
            protocol_version: ActionProtocolVersion::current(),
            idempotency_id: envelope.idempotency_id.clone(),
            colony_id: envelope.colony_id.clone(),
            result: LeaderAiActionResult::Accepted {
                accepted: ActionAcceptedResult {
                    result_code: report_safe("committed"),
                    changed_ids: vec![
                        BoundedEntityId::new(envelope.idempotency_id.as_str().to_owned())
                            .map_err(|_| ServerActionConflict::MalformedActionId)?,
                    ],
                    committed_versions,
                    current_state_hint: Some(CurrentStateHint {
                        state_code: report_safe("committed"),
                        visible_entity_id: None,
                        visible_stage: None,
                    }),
                },
            },
            refresh: None,
        })
    }
}

#[cfg(any())]
fn apply_live_leader_ai_action(
    world: &mut WorldState,
    envelope: &LeaderAiActionEnvelope,
    session: &VerifiedPlayerSession,
    now_ms: i64,
) -> Result<(), ServerActionConflict> {
    match &envelope.payload {
        cat_protocol::LeaderAiActionPayload::SelectColony { target_colony_id } => {
            let target = world
                .colonies
                .iter()
                .find(|colony| colony.id == target_colony_id.as_str())
                .ok_or(ServerActionConflict::OpaqueExistenceDenied)?;
            let can_control = target.kind == VillageKind::Global
                || target.owner_player_id.as_deref() == Some(session.player_id().as_str());
            if can_control {
                Ok(())
            } else {
                Err(ServerActionConflict::OpaqueExistenceDenied)
            }
        }
        cat_protocol::LeaderAiActionPayload::FoundVillage { display_name } => {
            let action = ClientAction::FoundVillage {
                name: display_name.as_str().to_owned(),
                session_id: session.session_id().to_owned(),
                sig: None,
            };
            let ctx = ActionCtx {
                session_id: session.session_id().to_owned(),
                player_id: session.player_id().as_str().to_owned(),
                colony_id: envelope.colony_id.as_str().to_owned(),
                now_ms,
            };
            let result = apply_action(world, &action, &ctx);
            if result.ok {
                Ok(())
            } else {
                Err(ServerActionConflict::PreconditionFailed(report_safe(
                    "village_foundation_rejected",
                )))
            }
        }
        cat_protocol::LeaderAiActionPayload::NudgePlan { plan_id, nudge, .. } => {
            let colony = selected_colony_mut(world, envelope)?;
            let intent_id = colony
                .leader_ai_runtime
                .intents
                .iter()
                .find(|(id, intent)| {
                    id.as_str() == plan_id.as_str() && !intent.lifecycle.state.is_terminal()
                })
                .map(|(id, _)| id.clone())
                .ok_or_else(|| precondition("plan_unavailable"))?;
            let scheduler = &mut colony.leader_ai_runtime.scheduling.scheduler;
            scheduler
                .advance_epoch(colony.leader_ai_runtime.planner.planning_epoch)
                .map_err(|_| precondition("planner_epoch_invalid"))?;
            let action = if nudge.get() > 0 {
                cat_sim::scheduler::PlayerEpochAction::MoveUp
            } else {
                cat_sim::scheduler::PlayerEpochAction::MoveDown
            };
            if matches!(
                scheduler.set_player_influence(intent_id, action),
                cat_sim::scheduler::InfluenceUpdate::CapacityReached
            ) {
                return Err(precondition("planner_influence_capacity"));
            }
            Ok(())
        }
        cat_protocol::LeaderAiActionPayload::DismissIntent {
            intent_id,
            planning_epoch,
            ..
        } => {
            let colony = selected_colony_mut(world, envelope)?;
            if *planning_epoch != colony.leader_ai_runtime.planner.planning_epoch {
                return Err(precondition("planner_epoch_changed"));
            }
            let intent_id = colony
                .leader_ai_runtime
                .intents
                .iter()
                .find(|(id, intent)| {
                    id.as_str() == intent_id.as_str()
                        && !intent.lifecycle.state.is_terminal()
                        && !matches!(
                            intent.authority_domain,
                            cat_sim::authority::AuthorityDomain::Survival
                                | cat_sim::authority::AuthorityDomain::Evacuation
                        )
                })
                .map(|(id, _)| id.clone())
                .ok_or_else(|| precondition("intent_not_dismissible"))?;
            let scheduler = &mut colony.leader_ai_runtime.scheduling.scheduler;
            scheduler
                .advance_epoch(*planning_epoch)
                .map_err(|_| precondition("planner_epoch_invalid"))?;
            if matches!(
                scheduler.set_player_influence(
                    intent_id,
                    cat_sim::scheduler::PlayerEpochAction::Dismiss,
                ),
                cat_sim::scheduler::InfluenceUpdate::CapacityReached
            ) {
                return Err(precondition("planner_influence_capacity"));
            }
            Ok(())
        }
        cat_protocol::LeaderAiActionPayload::CreateStandingOrder {
            order_kind,
            domain,
            target_id,
            instruction,
            priority_basis_points,
            expires_at_ms,
        } => {
            let colony = selected_colony_mut(world, envelope)?;
            let colony_id =
                cat_sim::planner_core::PlannerId::derive("colony", [colony.id.as_str()]);
            let effects = cat_sim::scholar_research::ResearchTrackStages::from_progress(
                &colony.leader_ai_runtime.research.purchases,
            )
            .map_err(|_| precondition("research_state_invalid"))?
            .effects();
            let created_tick = runtime_tick_at(colony, now_ms);
            let expires_tick =
                expires_at_ms.map(|expires| runtime_expiry_tick(colony, now_ms, expires));
            colony
                .leader_ai_runtime
                .player_directives
                .create_standing_order(
                    cat_sim::player_directives::StandingOrder {
                        id: cat_sim::player_directives::PlayerDirectiveId::derive(
                            "standing_order",
                            &colony_id,
                            envelope.idempotency_id.as_str(),
                        ),
                        order_kind: order_kind.as_str().to_owned(),
                        domain: domain.as_str().to_owned(),
                        target_id: target_id.as_ref().map(|id| id.as_str().to_owned()),
                        instruction: instruction.as_str().to_owned(),
                        priority_basis_points: priority_basis_points.get(),
                        expires_tick,
                        created_tick,
                    },
                    usize::from(effects.standing_order_slots),
                )
                .map_err(map_directive_error)
        }
        cat_protocol::LeaderAiActionPayload::UpdateStandingOrder {
            standing_order_id,
            patch,
        } => {
            let colony = selected_colony_mut(world, envelope)?;
            let canonical_id = colony
                .leader_ai_runtime
                .player_directives
                .standing_orders
                .values()
                .find(|order| {
                    leader_ai_snapshot_projection::stable_id_matches(
                        order.id.as_str(),
                        standing_order_id.as_str(),
                    )
                })
                .map(|order| order.id.as_str().to_owned())
                .ok_or_else(|| precondition("directive_unavailable"))?;
            colony
                .leader_ai_runtime
                .player_directives
                .update_standing_order(
                    canonical_id.as_str(),
                    cat_sim::player_directives::StandingOrderPatch {
                        instruction: patch
                            .instruction
                            .as_ref()
                            .map(|value| value.as_str().to_owned()),
                        priority_basis_points: patch
                            .priority_basis_points
                            .map(cat_protocol::BoundedBasisPoints::get),
                        target_id: patch
                            .target_id
                            .as_ref()
                            .map(|value| value.as_str().to_owned()),
                        clear_target: patch.clear_target,
                        expires_tick: patch.expires_at_ms.map(tick_from_ms),
                        clear_expiry: patch.clear_expiry,
                    },
                )
                .map_err(map_directive_error)
        }
        cat_protocol::LeaderAiActionPayload::DeleteStandingOrder { standing_order_id } => {
            let colony = selected_colony_mut(world, envelope)?;
            let canonical_id = colony
                .leader_ai_runtime
                .player_directives
                .standing_orders
                .values()
                .find(|order| {
                    leader_ai_snapshot_projection::stable_id_matches(
                        order.id.as_str(),
                        standing_order_id.as_str(),
                    )
                })
                .map(|order| order.id.as_str().to_owned())
                .ok_or_else(|| precondition("directive_unavailable"))?;
            colony
                .leader_ai_runtime
                .player_directives
                .delete_standing_order(canonical_id.as_str())
                .map_err(map_directive_error)
        }
        cat_protocol::LeaderAiActionPayload::AppointOfficer { role, cat_id } => {
            let colony = selected_colony_mut(world, envelope)?;
            if !colony
                .cats
                .iter()
                .any(|cat| cat.id == cat_id.as_str() && cat.death_time.is_none())
            {
                return Err(ServerActionConflict::PreconditionFailed(report_safe(
                    "candidate_unavailable",
                )));
            }
            let role = simulation_officer_role(*role);
            let cat = cat_sim::planner_core::PlannerId::derive("cat", [cat_id.as_str()]);
            colony
                .leader_ai_runtime
                .officers
                .institution
                .appoint_officer(role, cat, runtime_tick_at(colony, now_ms))
                .map_err(|_| {
                    ServerActionConflict::PreconditionFailed(report_safe("office_unavailable"))
                })?;
            colony.officers.insert(role, cat_id.as_str().to_owned());
            Ok(())
        }
        cat_protocol::LeaderAiActionPayload::UnappointOfficer { role } => {
            let colony = selected_colony_mut(world, envelope)?;
            let role = simulation_officer_role(*role);
            colony
                .leader_ai_runtime
                .officers
                .institution
                .vacate_office(role, runtime_tick_at(colony, now_ms))
                .map_err(|_| {
                    ServerActionConflict::PreconditionFailed(report_safe("office_unavailable"))
                })?;
            colony.officers.remove(&role);
            Ok(())
        }
        cat_protocol::LeaderAiActionPayload::OfficerAuthorityOverride {
            role,
            domain,
            request_id,
            mode,
        } => selected_colony_mut(world, envelope)?
            .leader_ai_runtime
            .player_directives
            .set_authority_override(
                cat_sim::player_directives::AuthorityOverrideKey {
                    role: simulation_officer_role(*role),
                    domain: domain.as_str().to_owned(),
                    request_id: request_id.as_ref().map(|id| id.as_str().to_owned()),
                },
                matches!(mode, cat_protocol::OfficerAuthorityMode::Grant),
            )
            .map_err(map_directive_error),
        cat_protocol::LeaderAiActionPayload::RequestTreatment {
            cat_id,
            injury_id,
            treatment_kind,
        } => {
            let colony = selected_colony_mut(world, envelope)?;
            if !colony
                .cats
                .iter()
                .any(|cat| cat.id == cat_id.as_str() && cat.death_time.is_none())
            {
                return Err(precondition("patient_unavailable"));
            }
            let colony_id =
                cat_sim::planner_core::PlannerId::derive("colony", [colony.id.as_str()]);
            colony
                .leader_ai_runtime
                .player_directives
                .request_treatment(cat_sim::player_directives::TreatmentRequest {
                    id: cat_sim::player_directives::PlayerDirectiveId::derive(
                        "treatment",
                        &colony_id,
                        envelope.idempotency_id.as_str(),
                    ),
                    cat_id: cat_id.as_str().to_owned(),
                    injury_id: injury_id.as_str().to_owned(),
                    treatment_kind: treatment_kind.as_str().to_owned(),
                    requested_tick: runtime_tick_at(colony, now_ms),
                })
                .map_err(map_directive_error)
        }
        cat_protocol::LeaderAiActionPayload::FitProsthetic {
            cat_id,
            prosthetic_id,
            body_part_id,
            fitting_site,
            fitter_cat_id,
        } => fit_prosthetic(
            selected_colony_mut(world, envelope)?,
            envelope,
            cat_id.as_str(),
            prosthetic_id.as_str(),
            body_part_id.as_str(),
            fitting_site,
            fitter_cat_id
                .as_ref()
                .map(cat_protocol::BoundedEntityId::as_str),
        ),
        cat_protocol::LeaderAiActionPayload::RepairProsthetic {
            prosthetic_id,
            workshop_id,
            input_reservation_id,
        } => repair_prosthetic(
            selected_colony_mut(world, envelope)?,
            prosthetic_id.as_str(),
            workshop_id.as_str(),
            input_reservation_id.as_str(),
        ),
        cat_protocol::LeaderAiActionPayload::PurchaseResearchWithFavor {
            study_id,
            use_preparation,
            displayed_price_micro_favor,
        } => purchase_research(
            selected_colony_mut(world, envelope)?,
            envelope,
            study_id.as_str(),
            *use_preparation,
            displayed_price_micro_favor.map(cat_protocol::BoundedFavorAmount::get),
            now_ms,
        ),
        cat_protocol::LeaderAiActionPayload::PrepareScholarStudy {
            study_id,
            scholar_cat_id,
        } => prepare_scholar_study(
            selected_colony_mut(world, envelope)?,
            envelope,
            study_id.as_str(),
            scholar_cat_id.as_str(),
            now_ms,
        ),
        cat_protocol::LeaderAiActionPayload::ActivateDivineBoost {
            boost_kind,
            duration_hours,
            displayed_price_micro_favor,
        } => activate_divine_boost(
            selected_colony_mut(world, envelope)?,
            envelope,
            session,
            boost_kind.as_str(),
            u32::from(*duration_hours),
            displayed_price_micro_favor.map(cat_protocol::BoundedFavorAmount::get),
            now_ms,
        ),
        cat_protocol::LeaderAiActionPayload::ChangeDiplomacy {
            target_colony_id,
            relationship,
        } => change_diplomacy(
            selected_colony_mut(world, envelope)?,
            envelope,
            session,
            target_colony_id.as_str(),
            Some(*relationship),
            None,
        ),
        cat_protocol::LeaderAiActionPayload::ApproveAlliance {
            target_colony_id,
            proposal_id,
        } => change_diplomacy(
            selected_colony_mut(world, envelope)?,
            envelope,
            session,
            target_colony_id.as_str(),
            None,
            Some(proposal_id.as_str()),
        ),
        cat_protocol::LeaderAiActionPayload::BlockColony {
            target_colony_id, ..
        } => change_diplomacy(
            selected_colony_mut(world, envelope)?,
            envelope,
            session,
            target_colony_id.as_str(),
            None,
            None,
        ),
        cat_protocol::LeaderAiActionPayload::AcceptTradeContract { contract_id } => {
            mutate_trade_contract(
                selected_colony_mut(world, envelope)?,
                envelope,
                session,
                contract_id.as_str(),
                cat_sim::autonomous_trade::TradeActionKind::Accept,
                now_ms,
            )
        }
        cat_protocol::LeaderAiActionPayload::RejectTradeContract { contract_id, .. } => {
            mutate_trade_contract(
                selected_colony_mut(world, envelope)?,
                envelope,
                session,
                contract_id.as_str(),
                cat_sim::autonomous_trade::TradeActionKind::Cancel,
                now_ms,
            )
        }
        cat_protocol::LeaderAiActionPayload::PhysicalPlacement { placement } => {
            let action =
                translate_physical_placement(placement, session, envelope.player_id.as_str())?;
            let ctx = ActionCtx {
                session_id: session.session_id().to_owned(),
                player_id: session.player_id().as_str().to_owned(),
                colony_id: envelope.colony_id.as_str().to_owned(),
                now_ms,
            };
            let result = apply_action(world, &action, &ctx);
            if result.ok {
                Ok(())
            } else {
                Err(ServerActionConflict::PreconditionFailed(report_safe(
                    "placement_rejected",
                )))
            }
        }
    }
}

fn precondition(code: &str) -> ServerActionConflict {
    ServerActionConflict::PreconditionFailed(report_safe(code))
}

#[cfg(any())]
fn map_directive_error(
    error: cat_sim::player_directives::PlayerDirectiveError,
) -> ServerActionConflict {
    use cat_sim::player_directives::PlayerDirectiveError;
    let code = match error {
        PlayerDirectiveError::UnknownDirective => "directive_unavailable",
        PlayerDirectiveError::InvalidDirective => "directive_invalid",
        PlayerDirectiveError::CapacityReached => "directive_capacity",
        PlayerDirectiveError::IdConflict => "directive_id_conflict",
        PlayerDirectiveError::VersionExhausted => "directive_version_exhausted",
        PlayerDirectiveError::MalformedPersistence => "directive_state_invalid",
    };
    precondition(code)
}

#[allow(clippy::too_many_arguments)]
#[cfg(any())]
fn fit_prosthetic(
    colony: &mut ColonyRuntime,
    envelope: &LeaderAiActionEnvelope,
    cat_id: &str,
    prosthetic_id: &str,
    body_part_id: &str,
    fitting_site: &cat_protocol::SiteRefActionTarget,
    fitter_cat_id: Option<&str>,
) -> Result<(), ServerActionConflict> {
    let item_id = colony
        .leader_ai_runtime
        .prosthetics
        .item_ids()
        .find(|id| id.as_str() == prosthetic_id)
        .cloned()
        .ok_or_else(|| precondition("prosthetic_unavailable"))?;
    let part = simulation_body_part(body_part_id)?;
    let patient = colony
        .leader_ai_runtime
        .cats
        .get(cat_id)
        .ok_or_else(|| precondition("patient_unavailable"))?;
    let fitter_id = fitter_cat_id.unwrap_or(cat_id);
    if !colony
        .cats
        .iter()
        .any(|cat| cat.id == fitter_id && cat.death_time.is_none())
    {
        return Err(precondition("fitter_unavailable"));
    }
    let anchor = target_anchor(fitting_site).ok_or_else(|| precondition("fitting_site_invalid"))?;
    let site_reachable = colony.revealed_tiles.contains(&TilePos {
        x: anchor.x,
        y: anchor.y,
    });
    let reservation_id = cat_sim::planner_core::PlannerId::derive(
        "prosthetic_fitting",
        [
            colony.id.as_str(),
            envelope.idempotency_id.as_str(),
            prosthetic_id,
        ],
    );
    let site_id = format!("tile:{}:{}", anchor.x, anchor.y);
    colony
        .leader_ai_runtime
        .prosthetics
        .begin_fitting(
            &item_id,
            &patient.anatomy,
            cat_sim::prosthetics::FitAuthorization {
                colony_id: &colony.id,
                cat_id,
                part,
                reservation_id: reservation_id.as_str(),
                fitter_id,
                fitter_capable: true,
                patient_consents: true,
                site_id: &site_id,
                site_kind: cat_sim::prosthetics::FitSiteKind::Treatment,
                site_reachable,
            },
        )
        .map_err(|_| precondition("prosthetic_fit_rejected"))
}

#[cfg(any())]
fn repair_prosthetic(
    colony: &mut ColonyRuntime,
    prosthetic_id: &str,
    workshop_id: &str,
    reservation_id: &str,
) -> Result<(), ServerActionConflict> {
    let item_id = colony
        .leader_ai_runtime
        .prosthetics
        .item_ids()
        .find(|id| id.as_str() == prosthetic_id)
        .cloned()
        .ok_or_else(|| precondition("prosthetic_unavailable"))?;
    let workshop_reachable = colony.buildings.iter().any(|building| {
        building.id == workshop_id
            && building.is_complete
            && building.building_type == cat_sim::types::BuildingType::Workshop
    });
    colony
        .leader_ai_runtime
        .prosthetics
        .begin_repair(
            &item_id,
            cat_sim::prosthetics::RepairAuthorization {
                colony_id: &colony.id,
                reservation_id,
                workshop_id,
                workshop_reachable,
                finite_inputs_authorized: !reservation_id.is_empty(),
            },
        )
        .map_err(|_| precondition("prosthetic_repair_rejected"))
}

#[cfg(any())]
fn simulation_body_part(value: &str) -> Result<cat_sim::anatomy::BodyPart, ServerActionConflict> {
    cat_sim::anatomy::BodyPart::ALL
        .into_iter()
        .find(|part| part.stable_id() == value)
        .ok_or_else(|| precondition("body_part_invalid"))
}

#[cfg(any())]
fn purchase_research(
    colony: &mut ColonyRuntime,
    envelope: &LeaderAiActionEnvelope,
    study_id: &str,
    use_preparation: bool,
    displayed_price: Option<u64>,
    now_ms: i64,
) -> Result<(), ServerActionConflict> {
    let catalog = cat_sim::research_purchase::canonical_research_catalog();
    let study_id = catalog
        .studies
        .iter()
        .find(|study| study.id.as_str() == study_id)
        .map(|study| study.id.clone())
        .ok_or_else(|| precondition("study_unavailable"))?;
    let study = catalog
        .study(&study_id)
        .ok_or_else(|| precondition("study_unavailable"))?;
    let expected_price = if use_preparation {
        study.undiscounted_price.micro_favor() * 3 / 4
    } else {
        study.undiscounted_price.micro_favor()
    };
    if displayed_price.is_some_and(|displayed| displayed != expected_price) {
        return Err(precondition("research_price_changed"));
    }
    let colony_id = cat_sim::planner_core::PlannerId::derive("colony", [colony.id.as_str()]);
    let request = cat_sim::scholar_research::ScholarPlayerPurchaseRequest {
        id: cat_sim::research_purchase::ResearchPurchaseId::derive(
            "player",
            &colony_id,
            envelope.idempotency_id.as_str(),
        ),
        colony_id,
        study_id,
        expected_research_version: colony.leader_ai_runtime.research.purchases.version,
        expected_favor_version: colony.leader_ai_runtime.shrine_favor.favor.version,
        expected_scholar_version: colony.leader_ai_runtime.research.scholars.version,
        use_preparation,
        now_tick: runtime_tick_at(colony, now_ms),
    };
    let runtime = &mut colony.leader_ai_runtime;
    runtime
        .research
        .scholars
        .player_purchase(
            &mut runtime.research.purchases,
            &mut runtime.shrine_favor.favor,
            catalog,
            request,
        )
        .map(|_| ())
        .map_err(map_scholar_error)
}

#[cfg(any())]
fn prepare_scholar_study(
    colony: &mut ColonyRuntime,
    envelope: &LeaderAiActionEnvelope,
    study_id: &str,
    scholar_cat_id: &str,
    now_ms: i64,
) -> Result<(), ServerActionConflict> {
    let colony_id = cat_sim::planner_core::PlannerId::derive("colony", [colony.id.as_str()]);
    let catalog = cat_sim::research_purchase::canonical_research_catalog();
    let study_id = catalog
        .studies
        .iter()
        .find(|study| study.id.as_str() == study_id)
        .map(|study| study.id.clone())
        .ok_or_else(|| precondition("study_unavailable"))?;
    colony
        .leader_ai_runtime
        .research
        .scholars
        .prepare_study(
            catalog,
            &colony.leader_ai_runtime.research.purchases,
            cat_sim::scholar_research::PrepareStudyRequest {
                id: cat_sim::scholar_research::PreparationId::derive(
                    &colony_id,
                    envelope.idempotency_id.as_str(),
                ),
                study_id,
                assigned_scholar: cat_sim::scholar_research::ScholarId::derive(scholar_cat_id),
                expected_version: colony.leader_ai_runtime.research.scholars.version,
                prepared_tick: runtime_tick_at(colony, now_ms),
            },
        )
        .map(|_| ())
        .map_err(map_scholar_error)
}

#[cfg(any())]
fn map_scholar_error(
    error: cat_sim::scholar_research::ScholarResearchError,
) -> ServerActionConflict {
    use cat_sim::{
        favor::FavorError, research_purchase::ResearchPurchaseError,
        scholar_research::ScholarResearchError,
    };
    if matches!(
        error,
        ScholarResearchError::Purchase(ResearchPurchaseError::Favor(FavorError::InsufficientFavor))
    ) {
        return ServerActionConflict::InsufficientFavor(Box::new(CurrentStateHint {
            state_code: report_safe("insufficient_favor"),
            visible_entity_id: None,
            visible_stage: None,
        }));
    }
    let code = match error {
        ScholarResearchError::UnknownScholar | ScholarResearchError::ScholarDead => {
            "scholar_unavailable"
        }
        ScholarResearchError::UnknownStudy => "study_unavailable",
        ScholarResearchError::StudyAlreadyOwned => "study_already_owned",
        ScholarResearchError::AlreadyPrepared => "study_already_prepared",
        ScholarResearchError::PreparationNotFound => "preparation_unavailable",
        ScholarResearchError::InsufficientInsight => "insufficient_insight",
        ScholarResearchError::Purchase(ResearchPurchaseError::NotFrontier) => "study_not_frontier",
        ScholarResearchError::Purchase(ResearchPurchaseError::AlreadyOwned) => {
            "study_already_owned"
        }
        _ => "research_precondition_failed",
    };
    precondition(code)
}

#[cfg(any())]
fn activate_divine_boost(
    colony: &mut ColonyRuntime,
    envelope: &LeaderAiActionEnvelope,
    session: &VerifiedPlayerSession,
    boost_kind: &str,
    duration_hours: u32,
    displayed_price: Option<u64>,
    now_ms: i64,
) -> Result<(), ServerActionConflict> {
    let boost_type = match boost_kind {
        "bountiful_labor" => cat_sim::divine_boosts::DivineBoostType::BountifulLabor,
        "fleet_paws" => cat_sim::divine_boosts::DivineBoostType::FleetPaws,
        "inspired_work" => cat_sim::divine_boosts::DivineBoostType::InspiredWork,
        "restorative_grace" => cat_sim::divine_boosts::DivineBoostType::RestorativeGrace,
        _ => return Err(precondition("boost_kind_invalid")),
    };
    let stages = cat_sim::scholar_research::ResearchTrackStages::from_progress(
        &colony.leader_ai_runtime.research.purchases,
    )
    .map_err(|_| precondition("research_state_invalid"))?
    .effects()
    .divine_boost_stages;
    let cost = cat_sim::divine_boosts::boost_cost(boost_type, duration_hours, stages)
        .map_err(|_| precondition("boost_duration_locked"))?;
    if displayed_price.is_some_and(|displayed| displayed != cost.micro_favor()) {
        return Err(precondition("boost_price_changed"));
    }
    let colony_id = cat_sim::planner_core::PlannerId::derive("colony", [colony.id.as_str()]);
    let player_id =
        cat_sim::planner_core::PlannerId::derive("player", [session.player_id().as_str()]);
    let activated_tick = runtime_tick_at(colony, now_ms);
    let runtime = &mut colony.leader_ai_runtime;
    let expected_favor_version = runtime.shrine_favor.favor.version;
    runtime
        .boosts
        .purchase(
            &mut runtime.shrine_favor.favor,
            cat_sim::divine_boosts::DivineBoostPurchaseRequest {
                id: cat_sim::divine_boosts::DivineBoostPurchaseId::derive(
                    "player",
                    &colony_id,
                    envelope.idempotency_id.as_str(),
                ),
                colony_id,
                actor: cat_sim::authority::AuthorityActor::God {
                    player_id: player_id.clone(),
                },
                authority_context: cat_sim::authority::AuthorityContext {
                    leader_present: runtime.officers.institution.leader().is_some(),
                    player_authorized: true,
                },
                boost_type,
                duration_hours,
                committed_research_stages: stages,
                expected_boost_version: runtime.boosts.version,
                expected_favor_version,
                activated_tick,
                ticks_per_game_hour: 60,
            },
        )
        .map(|_| ())
        .map_err(|error| {
            if matches!(
                error,
                cat_sim::divine_boosts::DivineBoostError::Favor(
                    cat_sim::favor::FavorError::InsufficientFavor
                )
            ) {
                ServerActionConflict::InsufficientFavor(Box::new(CurrentStateHint {
                    state_code: report_safe("insufficient_favor"),
                    visible_entity_id: None,
                    visible_stage: None,
                }))
            } else {
                precondition("boost_precondition_failed")
            }
        })
}

#[cfg(any())]
fn change_diplomacy(
    colony: &mut ColonyRuntime,
    envelope: &LeaderAiActionEnvelope,
    session: &VerifiedPlayerSession,
    target_colony_id: &str,
    proposed: Option<cat_protocol::DiplomacyRelationshipTarget>,
    proposal_id: Option<&str>,
) -> Result<(), ServerActionConflict> {
    let acting = cat_sim::diplomacy::DiplomacyColonyId::derive(&colony.id);
    let target = cat_sim::diplomacy::DiplomacyColonyId::derive(target_colony_id);
    let pair = cat_sim::diplomacy::DiplomacyPair::new(acting.clone(), target)
        .map_err(|_| precondition("diplomacy_target_invalid"))?;
    let current_version = colony
        .leader_ai_runtime
        .diplomacy
        .record(pair.id())
        .map_or(0, |record| record.version);
    if let Some(expected_proposal) = proposal_id {
        let matches_pending = colony
            .leader_ai_runtime
            .diplomacy
            .record(pair.id())
            .and_then(|record| record.pending_consent.as_ref())
            .is_some_and(|pending| pending.proposal_action_id.as_str() == expected_proposal);
        if !matches_pending {
            return Err(precondition("alliance_proposal_unavailable"));
        }
    }
    let kind = match (proposed, proposal_id) {
        (Some(cat_protocol::DiplomacyRelationshipTarget::Friendly), _) => {
            cat_sim::diplomacy::DiplomacyActionKind::Propose(
                cat_sim::diplomacy::ProposedRelationship::Friendly,
            )
        }
        (Some(cat_protocol::DiplomacyRelationshipTarget::Allied), _) => {
            cat_sim::diplomacy::DiplomacyActionKind::Propose(
                cat_sim::diplomacy::ProposedRelationship::Allied,
            )
        }
        (None, Some(_)) => cat_sim::diplomacy::DiplomacyActionKind::Approve,
        (None, None) => cat_sim::diplomacy::DiplomacyActionKind::Block,
    };
    let player_id =
        cat_sim::planner_core::PlannerId::derive("player", [session.player_id().as_str()]);
    colony
        .leader_ai_runtime
        .diplomacy
        .apply(
            cat_sim::diplomacy::DiplomacyAction {
                id: cat_sim::diplomacy::DiplomacyActionId::derive(
                    pair.id(),
                    &acting,
                    envelope.idempotency_id.as_str(),
                ),
                pair,
                acting_colony_id: acting.clone(),
                expected_version: current_version,
                kind,
            },
            cat_sim::diplomacy::DiplomacyAuthorization {
                actor: cat_sim::authority::AuthorityActor::God {
                    player_id: player_id.clone(),
                },
                acting_colony_id: acting,
                owner_player_id: player_id,
                player_authorized: true,
            },
        )
        .map(|_| ())
        .map_err(|_| precondition("diplomacy_precondition_failed"))
}

#[cfg(any())]
fn mutate_trade_contract(
    colony: &mut ColonyRuntime,
    envelope: &LeaderAiActionEnvelope,
    session: &VerifiedPlayerSession,
    contract_id: &str,
    kind: cat_sim::autonomous_trade::TradeActionKind,
    now_ms: i64,
) -> Result<(), ServerActionConflict> {
    let contract = colony
        .leader_ai_runtime
        .trade
        .contracts()
        .find(|contract| {
            leader_ai_snapshot_projection::stable_id_matches(contract.id().as_str(), contract_id)
        })
        .cloned()
        .ok_or_else(|| precondition("trade_contract_unavailable"))?;
    let acting = cat_sim::diplomacy::DiplomacyColonyId::derive(&colony.id);
    let player_id =
        cat_sim::planner_core::PlannerId::derive("player", [session.player_id().as_str()]);
    let action = cat_sim::autonomous_trade::TradeAction {
        id: cat_sim::autonomous_trade::TradeActionId::derive(
            contract.id(),
            &acting,
            envelope.idempotency_id.as_str(),
            kind,
        ),
        contract_id: contract.id().clone(),
        acting_colony: acting.clone(),
        expected_version: contract.version,
        kind,
    };
    let relationship = colony
        .leader_ai_runtime
        .diplomacy
        .relationship(&contract.proposal.pair);
    let now_tick = runtime_tick_at(colony, now_ms);
    let runtime = &mut colony.leader_ai_runtime;
    runtime
        .trade
        .apply_action(
            action,
            &cat_sim::autonomous_trade::TradeAuthorization {
                actor: cat_sim::authority::AuthorityActor::God {
                    player_id: player_id.clone(),
                },
                acting_colony: acting,
                owner_player_id: Some(player_id),
                authorized_for_colony: true,
            },
            relationship,
            now_tick,
            &mut runtime.scheduling.world_reservations,
        )
        .map(|_| ())
        .map_err(|error| {
            warn!(
                contract_id,
                ?kind,
                ?error,
                "leader-AI trade mutation failed after authenticated preflight"
            );
            if matches!(error, cat_sim::autonomous_trade::TradeError::Escrow(_)) {
                ServerActionConflict::ReservationConflict(Box::new(CurrentStateHint {
                    state_code: report_safe("trade_escrow_conflict"),
                    visible_entity_id: None,
                    visible_stage: None,
                }))
            } else {
                let reason = match error {
                    cat_sim::autonomous_trade::TradeError::Expired => "trade_expired",
                    cat_sim::autonomous_trade::TradeError::RelationshipDenied => {
                        "trade_relationship_denied"
                    }
                    cat_sim::autonomous_trade::TradeError::AuthorizationDenied(_)
                    | cat_sim::autonomous_trade::TradeError::AuthorizationColonyMismatch
                    | cat_sim::autonomous_trade::TradeError::PlayerIdentityMismatch => {
                        "trade_authorization_denied"
                    }
                    cat_sim::autonomous_trade::TradeError::ContractNotFound => {
                        "trade_contract_unavailable"
                    }
                    cat_sim::autonomous_trade::TradeError::StaleVersion { .. } => {
                        "trade_contract_stale"
                    }
                    cat_sim::autonomous_trade::TradeError::InvalidTransition => {
                        "trade_transition_invalid"
                    }
                    _ => "trade_precondition_failed",
                };
                precondition(reason)
            }
        })
}

#[cfg(any())]
fn selected_colony_mut<'a>(
    world: &'a mut WorldState,
    envelope: &LeaderAiActionEnvelope,
) -> Result<&'a mut ColonyRuntime, ServerActionConflict> {
    world
        .colonies
        .iter_mut()
        .find(|colony| colony.id == envelope.colony_id.as_str())
        .ok_or(ServerActionConflict::OpaqueExistenceDenied)
}

#[cfg(any())]
fn simulation_officer_role(role: cat_protocol::OfficerRole) -> cat_sim::officers::OfficerRole {
    match role {
        cat_protocol::OfficerRole::Steward => cat_sim::officers::OfficerRole::Steward,
        cat_protocol::OfficerRole::Accountant => cat_sim::officers::OfficerRole::Accountant,
        cat_protocol::OfficerRole::Forester => cat_sim::officers::OfficerRole::Forester,
        cat_protocol::OfficerRole::Farmer => cat_sim::officers::OfficerRole::Farmer,
        cat_protocol::OfficerRole::Captain => cat_sim::officers::OfficerRole::Captain,
        cat_protocol::OfficerRole::Loremaster => cat_sim::officers::OfficerRole::Loremaster,
        cat_protocol::OfficerRole::ClothLeader => cat_sim::officers::OfficerRole::ClothLeader,
    }
}

#[cfg(any())]
fn current_server_state_versions(
    world: &WorldState,
    colony_id: &str,
) -> Option<CurrentVersionHint> {
    let colony = world
        .colonies
        .iter()
        .find(|colony| colony.id == colony_id)?;
    let runtime = &colony.leader_ai_runtime;
    Some(CurrentVersionHint {
        planner_version: Some(stable_serialized_version(&(
            runtime.planner,
            &runtime.scheduling.scheduler,
        ))),
        domain_version: Some(runtime.beliefs.version),
        resource_version: Some(runtime.shrine_favor.favor.version),
        spatial_version: Some(runtime.scheduling.world_reservations.version()),
        reservation_version: Some(runtime.scheduling.reservations.version),
        research_version: Some(runtime.research.purchases.version),
        scholar_version: Some(runtime.research.scholars.version),
        boost_version: Some(runtime.boosts.version),
        diplomacy_version: Some(stable_serialized_version(&runtime.diplomacy)),
        trade_version: Some(runtime.trade.version()),
        prosthetic_version: Some(stable_serialized_version(&runtime.prosthetics)),
        care_version: Some(stable_serialized_version(&(
            &runtime.cats,
            &runtime.player_directives.treatment_requests,
        ))),
        officer_version: Some(stable_serialized_version(&(
            &runtime.officers.institution,
            &runtime.player_directives.authority_overrides,
        ))),
        standing_order_version: Some(runtime.player_directives.version),
    })
}

#[cfg(any())]
fn stable_serialized_version(value: &impl serde::Serialize) -> u64 {
    leader_ai_snapshot_projection::stable_serialized_version(value)
}

#[cfg(any())]
fn tick_from_ms(now_ms: i64) -> u64 {
    u64::try_from(now_ms.max(0)).unwrap_or_default()
}

#[cfg(any())]
fn runtime_tick_at(colony: &ColonyRuntime, now_ms: i64) -> u64 {
    let elapsed_ms = now_ms.saturating_sub(colony.run_started_at).max(0) as f64;
    let scale = if colony.test_time_scale.is_finite() {
        colony.test_time_scale.max(1.0)
    } else {
        1.0
    };
    let minutes = elapsed_ms * scale / 60_000.0;
    if minutes.is_finite() {
        minutes.floor().clamp(0.0, u64::MAX as f64) as u64
    } else {
        u64::MAX
    }
}

#[cfg(any())]
fn runtime_expiry_tick(colony: &ColonyRuntime, now_ms: i64, expires_at_ms: i64) -> u64 {
    let remaining_ms = expires_at_ms.saturating_sub(now_ms).max(0) as f64;
    let scale = if colony.test_time_scale.is_finite() {
        colony.test_time_scale.max(1.0)
    } else {
        1.0
    };
    runtime_tick_at(colony, now_ms).saturating_add((remaining_ms * scale / 60_000.0).ceil() as u64)
}

fn report_safe(value: &str) -> ReportSafeString {
    ReportSafeString::new(value).expect("server report-safe literals are bounded and non-empty")
}

fn translate_physical_placement(
    placement: &cat_protocol::PhysicalPlacementActionPayload,
    session: &VerifiedPlayerSession,
    nickname: &str,
) -> Result<ClientAction, ServerActionConflict> {
    use cat_protocol::PhysicalPlacementActionPayload as Placement;

    let session_id = session.session_id().to_owned();
    let nickname = nickname.to_owned();
    let sig = String::new();
    match placement {
        Placement::PlanBuilding {
            building_type,
            site,
        } => Ok(ClientAction::PlanBuilding {
            session_id,
            nickname,
            sig,
            building_type: *building_type,
            site: target_anchor(site),
        }),
        Placement::DesignateFarm { site, crop } => {
            let (a, b) = target_rectangle(site)?;
            Ok(ClientAction::DesignateFarm {
                session_id,
                nickname,
                sig,
                a,
                b,
                crop: *crop,
            })
        }
        Placement::DesignateStockpile { site, accepts } => {
            let (a, b) = target_rectangle(site)?;
            Ok(ClientAction::DesignateStockpile {
                session_id,
                nickname,
                sig,
                a,
                b,
                accepts: accepts.clone(),
            })
        }
        Placement::DesignateGatherSpot { site, resource } => {
            let (a, b) = target_rectangle(site)?;
            Ok(ClientAction::DesignateGatherSpot {
                session_id,
                nickname,
                sig,
                a,
                b,
                kind: *resource,
            })
        }
        Placement::DesignateFishingSpot { site } => Ok(ClientAction::DesignateFishingSpot {
            session_id,
            nickname,
            sig,
            at: target_exact(site)?,
        }),
        Placement::BuildRoad { route } => {
            let (a, b) = target_endpoints(route)?;
            Ok(ClientAction::BuildRoad {
                session_id,
                nickname,
                sig,
                a,
                b,
            })
        }
        Placement::BuildBridge { site } => Ok(ClientAction::BuildBridge {
            session_id,
            nickname,
            sig,
            at: target_exact(site)?,
        }),
        Placement::DesignateRail {
            route,
            worker_cat_id,
        } => {
            let (a, b) = target_endpoints(route)?;
            Ok(ClientAction::DesignateRail {
                session_id,
                nickname,
                sig,
                a,
                b,
                cat_id: worker_cat_id.as_str().to_owned(),
            })
        }
        Placement::BuildDock {
            endpoints,
            worker_cat_id,
        } => {
            let (land, water) = target_pair(endpoints)?;
            Ok(ClientAction::BuildDock {
                session_id,
                nickname,
                sig,
                land,
                water,
                cat_id: worker_cat_id.as_str().to_owned(),
            })
        }
        Placement::BuildTransportVehicle {
            mode,
            home,
            worker_cat_id,
        } => Ok(ClientAction::BuildTransportVehicle {
            session_id,
            nickname,
            sig,
            mode: *mode,
            home: target_exact(home)?,
            cat_id: worker_cat_id.as_str().to_owned(),
        }),
        Placement::CreateTransportRoute {
            mode,
            source_stockpile_id,
            destination_stockpile_id,
            resource,
            amount,
            route,
            worker_cat_id,
            repeat,
        } => Ok(ClientAction::CreateTransportRoute {
            session_id,
            nickname,
            sig,
            mode: *mode,
            source_stockpile_id: source_stockpile_id.as_str().to_owned(),
            destination_stockpile_id: destination_stockpile_id.as_str().to_owned(),
            resource: *resource,
            amount: amount.get() as f64,
            path: target_path(route)?,
            cat_id: worker_cat_id.as_str().to_owned(),
            repeat: *repeat,
        }),
    }
}

fn target_anchor(target: &cat_protocol::SiteRefActionTarget) -> Option<cat_protocol::TilePoint> {
    match target {
        cat_protocol::SiteRefActionTarget::ExactTile { tile }
        | cat_protocol::SiteRefActionTarget::AnchoredRect { anchor: tile, .. } => {
            Some((*tile).into())
        }
        cat_protocol::SiteRefActionTarget::OrderedPath { ordered_tiles } => {
            ordered_tiles.first().copied().map(Into::into)
        }
        cat_protocol::SiteRefActionTarget::EndpointPair { source, .. } => Some((*source).into()),
    }
}

fn target_exact(
    target: &cat_protocol::SiteRefActionTarget,
) -> Result<cat_protocol::TilePoint, ServerActionConflict> {
    match target {
        cat_protocol::SiteRefActionTarget::ExactTile { tile } => Ok((*tile).into()),
        _ => Err(ServerActionConflict::PreconditionFailed(report_safe(
            "exact_tile_required",
        ))),
    }
}

fn target_rectangle(
    target: &cat_protocol::SiteRefActionTarget,
) -> Result<(cat_protocol::TilePoint, cat_protocol::TilePoint), ServerActionConflict> {
    let cat_protocol::SiteRefActionTarget::AnchoredRect {
        anchor,
        width,
        height,
    } = target
    else {
        return Err(ServerActionConflict::PreconditionFailed(report_safe(
            "rectangle_required",
        )));
    };
    let width = i32::from(*width);
    let height = i32::from(*height);
    Ok((
        (*anchor).into(),
        cat_protocol::TilePoint {
            x: anchor.x + width - 1,
            y: anchor.y + height - 1,
        },
    ))
}

fn target_endpoints(
    target: &cat_protocol::SiteRefActionTarget,
) -> Result<(cat_protocol::TilePoint, cat_protocol::TilePoint), ServerActionConflict> {
    match target {
        cat_protocol::SiteRefActionTarget::OrderedPath { ordered_tiles } => {
            let first = ordered_tiles.first().copied();
            let last = ordered_tiles.last().copied();
            first.zip(last).map_or_else(
                || {
                    Err(ServerActionConflict::PreconditionFailed(report_safe(
                        "route_required",
                    )))
                },
                |(first, last)| Ok((first.into(), last.into())),
            )
        }
        cat_protocol::SiteRefActionTarget::EndpointPair {
            source,
            destination,
        } => Ok(((*source).into(), (*destination).into())),
        _ => Err(ServerActionConflict::PreconditionFailed(report_safe(
            "route_required",
        ))),
    }
}

fn target_pair(
    target: &cat_protocol::SiteRefActionTarget,
) -> Result<(cat_protocol::TilePoint, cat_protocol::TilePoint), ServerActionConflict> {
    let cat_protocol::SiteRefActionTarget::EndpointPair {
        source,
        destination,
    } = target
    else {
        return Err(ServerActionConflict::PreconditionFailed(report_safe(
            "endpoint_pair_required",
        )));
    };
    Ok(((*source).into(), (*destination).into()))
}

fn target_path(
    target: &cat_protocol::SiteRefActionTarget,
) -> Result<Vec<cat_protocol::TilePoint>, ServerActionConflict> {
    let cat_protocol::SiteRefActionTarget::OrderedPath { ordered_tiles } = target else {
        return Err(ServerActionConflict::PreconditionFailed(report_safe(
            "ordered_route_required",
        )));
    };
    Ok(ordered_tiles.iter().copied().map(Into::into).collect())
}

fn normalized_player_name(raw: &str) -> Result<Option<String>, String> {
    let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        // A nameless Presence may only bootstrap a signed session. Mutating
        // actions below require a later named Presence on the same socket.
        return Ok(None);
    }
    let count = normalized.chars().count();
    if count < 2 {
        return Err("Player name must be at least 2 characters long.".to_owned());
    }
    if count > PLAYER_NAME_MAX_CHARS {
        return Err(format!(
            "Player name must be at most {PLAYER_NAME_MAX_CHARS} characters long."
        ));
    }
    if normalized.chars().any(char::is_control) {
        return Err("Player name contains invalid characters.".to_owned());
    }
    Ok(Some(normalized))
}

fn embedded_action_nickname(action: &ClientAction) -> Option<String> {
    serde_json::to_value(action)
        .ok()?
        .get("nickname")?
        .as_str()
        .map(str::to_owned)
}

fn action_audit_message(action: &ClientAction) -> Option<String> {
    let value = serde_json::to_value(action).ok()?;
    let tag = value.get("action")?.as_str()?;
    if matches!(
        tag,
        "presence" | "ensure" | "setTestAcceleration" | "advanceTime" | "setTestRngSeed"
    ) {
        return None;
    }
    Some(humanize_camel_case(tag))
}

fn humanize_camel_case(value: &str) -> String {
    let mut label = String::with_capacity(value.len() + 8);
    for (index, character) in value.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if index > 0 {
                label.push(' ');
            }
            label.push(character.to_ascii_lowercase());
        } else if index == 0 {
            label.push(character.to_ascii_uppercase());
        } else {
            label.push(character);
        }
    }
    label
}

fn append_player_action_event(
    world: &mut WorldState,
    colony_id: &str,
    actor_name: &str,
    message: String,
    now_ms: i64,
) {
    let Some(colony) = world
        .colonies
        .iter_mut()
        .find(|colony| colony.id == colony_id)
    else {
        return;
    };
    colony.events.push(EventLog {
        id: format!("event-{}-{}", now_ms, colony.events.len() + 1),
        at_ms: now_ms,
        kind: EventKind::Other("player_action".to_owned()),
        message,
        actor_name: Some(actor_name.to_owned()),
    });
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
        | ClientAction::OfferResource {
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
        | ClientAction::BuildBridge {
            session_id, sig, ..
        }
        | ClientAction::DesignateRail {
            session_id, sig, ..
        }
        | ClientAction::BuildDock {
            session_id, sig, ..
        }
        | ClientAction::BuildTransportVehicle {
            session_id, sig, ..
        }
        | ClientAction::CreateTransportRoute {
            session_id, sig, ..
        }
        | ClientAction::CancelTransportRoute {
            session_id, sig, ..
        }
        | ClientAction::SellGoods {
            session_id, sig, ..
        }
        | ClientAction::RepairItem {
            session_id, sig, ..
        }
        | ClientAction::EquipItem {
            session_id, sig, ..
        }
        | ClientAction::UnequipItem {
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
        | ClientAction::EditProductionWorkSlot {
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

async fn send_leader_ai_snapshot(
    socket: &mut WebSocket,
    state: &AppState,
    connection: &ConnectionContext,
) -> Result<(), axum::Error> {
    let world = state.world.lock().await;
    let directory = state.village_directory.read().await;
    let projected = build_report_safe_leader_ai_snapshot(&world, &directory, connection, now_ms());
    drop(directory);
    drop(world);
    let serialized = match projected {
        Ok(snapshot) => serde_json::to_string(&snapshot),
        Err(error) => {
            error!(%error, "canonical selected-colony projection rejected before send");
            serde_json::to_string(&canonical_adapter_error(
                "snapshot:unavailable",
                "The selected village report is temporarily unavailable.",
            ))
        }
    }
    .expect("canonical protocol DTO serializes");
    socket.send(Message::Text(serialized.into())).await
}

fn build_report_safe_leader_ai_snapshot(
    world: &WorldState,
    directory: &BTreeMap<String, VillageDirectoryEntry>,
    connection: &ConnectionContext,
    now_ms: i64,
) -> Result<CanonicalSnapshotEnvelope, CanonicalWireError> {
    let identity = connection
        .identity
        .as_ref()
        .ok_or(CanonicalWireError::WrongPartition)?;
    let selected = world
        .colonies
        .iter()
        .find(|colony| colony.id == connection.colony_id)
        .ok_or(CanonicalWireError::WrongPartition)?;
    let known = &selected.known_village_ids;
    let public_villages = directory
        .iter()
        .filter(|(id, entry)| {
            *id == &selected.id || entry.kind == VillageKind::Global || known.contains(*id)
        })
        .map(|(id, entry)| {
            let is_owner = entry.owner_player_id.as_deref() == Some(identity.player_id.as_str());
            Ok(PublicColonySummaryV2 {
                colony_id: StableId::new(id.clone())?,
                display_name: ReportText::new(entry.name.clone())?,
                can_view: true,
                can_control: entry.kind == VillageKind::Global || is_owner,
            })
        })
        .collect::<Result<Vec<_>, CanonicalWireError>>()?;
    leader_ai_snapshot_projection::project_selected_colony(
        world,
        &selected.id,
        public_villages,
        identity.player_id.as_str(),
        now_ms,
    )
}

#[cfg(test)]
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

#[cfg(any())]
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
                trusted_proxies: hosting::TrustedProxies::default(),
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
            trusted_proxies: hosting::TrustedProxies::default(),
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

    #[tokio::test]
    async fn readiness_fails_after_repeated_persistence_failures() {
        let state = build_state(1_000_000);
        state
            .consecutive_save_failures
            .store(SAVE_FAILURES_BEFORE_NOT_READY, Ordering::SeqCst);

        let response = app(state, &local_config())
            .oneshot(request("/ready"))
            .await
            .expect("readiness response");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn periodic_save_failures_trip_and_success_resets_readiness_state() {
        let state = build_state(1_000_000);
        {
            let mut world = state.world.lock().await;
            let mut invalid_second_global = world.colonies[0].clone();
            invalid_second_global.id = "invalid-second-global".to_owned();
            world.colonies.push(invalid_second_global);
        }
        for attempt in 1..=SAVE_FAILURES_BEFORE_NOT_READY {
            run_tick_once(
                state.clone(),
                u64::from(attempt) * SAVE_EVERY_TICKS,
                1_000_000,
                |_, _| {},
            )
            .await
            .expect("save attempt worker");
        }
        assert_eq!(
            state.consecutive_save_failures.load(Ordering::SeqCst),
            SAVE_FAILURES_BEFORE_NOT_READY
        );

        state.world.lock().await.colonies.pop();
        run_tick_once(state.clone(), 20, 1_000_000, |_, _| {})
            .await
            .expect("recovery save worker");
        assert_eq!(state.consecutive_save_failures.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn peer_connection_registry_caps_and_releases_each_ip() {
        let registry = Arc::new(PeerConnections::default());
        let peer = IpAddr::from([192, 0, 2, 10]);
        let mut guards = (0..MAX_CONNECTIONS_PER_IP)
            .map(|_| registry.acquire(peer).expect("connection below cap"))
            .collect::<Vec<_>>();
        assert!(registry.acquire(peer).is_none());
        assert!(registry.acquire(IpAddr::from([192, 0, 2, 11])).is_some());

        guards.pop();
        assert!(registry.acquire(peer).is_some());
    }

    #[test]
    fn forwarding_headers_are_used_only_for_explicit_trusted_proxy_peers() {
        let proxy = IpAddr::from([10, 0, 0, 8]);
        let client = IpAddr::from([198, 51, 100, 42]);
        let trusted =
            hosting::TrustedProxies::parse(Some(proxy.to_string()), "CAT_SERVER_TRUSTED_PROXY_IPS")
                .expect("trusted proxy fixture");
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            client.to_string().parse().expect("forwarded header"),
        );

        assert_eq!(
            effective_peer_ip(proxy, &headers, &trusted),
            Ok(client),
            "configured proxy may supply the one effective client address"
        );
        assert_eq!(
            effective_peer_ip(IpAddr::from([203, 0, 113, 9]), &headers, &trusted),
            Ok(IpAddr::from([203, 0, 113, 9])),
            "ordinary clients cannot spoof their limiter identity"
        );
    }

    #[test]
    fn trusted_proxy_forwarding_header_is_strict_and_fail_closed() {
        let proxy = IpAddr::from([10, 0, 0, 8]);
        let trusted =
            hosting::TrustedProxies::parse(Some(proxy.to_string()), "CAT_SERVER_TRUSTED_PROXY_IPS")
                .expect("trusted proxy fixture");
        assert!(effective_peer_ip(proxy, &HeaderMap::new(), &trusted).is_err());

        let mut chained = HeaderMap::new();
        chained.insert(
            "x-forwarded-for",
            "198.51.100.42, 10.0.0.7".parse().expect("forwarded chain"),
        );
        assert!(effective_peer_ip(proxy, &chained, &trusted).is_err());

        let mut duplicate = HeaderMap::new();
        duplicate.append(
            "x-forwarded-for",
            "198.51.100.42".parse().expect("first forwarded value"),
        );
        duplicate.append(
            "x-forwarded-for",
            "198.51.100.43".parse().expect("second forwarded value"),
        );
        assert!(effective_peer_ip(proxy, &duplicate, &trusted).is_err());

        let mut malformed = HeaderMap::new();
        malformed.insert(
            "x-forwarded-for",
            "not-an-ip".parse().expect("malformed header value"),
        );
        assert!(effective_peer_ip(proxy, &malformed, &trusted).is_err());
    }

    #[tokio::test]
    async fn one_ip_cannot_mint_unbounded_sessions() {
        let state = build_state(1_000_000);
        for index in 0..SESSION_ISSUE_LIMIT_MAX {
            let mut connection = ConnectionContext::for_peer_ip(
                IpAddr::from([192, 0, 2, 20]),
                STARTER_COLONY_ID.to_owned(),
            );
            let result = send_action(
                &state,
                &mut connection,
                &ClientAction::Presence {
                    session_id: String::new(),
                    nickname: format!("Browser {index}"),
                    sig: None,
                },
            )
            .await;
            assert!(result.result.ok, "session {index}: {result:?}");
        }

        let mut ninth = ConnectionContext::for_peer_ip(
            IpAddr::from([192, 0, 2, 20]),
            STARTER_COLONY_ID.to_owned(),
        );
        let rejected = send_action(
            &state,
            &mut ninth,
            &ClientAction::Presence {
                session_id: String::new(),
                nickname: "Abusive browser".to_owned(),
                sig: None,
            },
        )
        .await;
        assert!(!rejected.result.ok);
        assert!(
            rejected
                .result
                .message
                .as_deref()
                .is_some_and(|message| message.contains("Too many new sessions"))
        );
    }

    #[tokio::test]
    async fn world_colony_cap_rejects_founding_before_simulation_growth() {
        let state = build_state(1_000_000);
        {
            let mut world = state.world.lock().await;
            let template = world.colonies[0].clone();
            while world.colonies.len() < MAX_TOTAL_COLONIES {
                let mut colony = template.clone();
                colony.id = format!("capacity-fixture-{}", world.colonies.len());
                colony.kind = VillageKind::Personal;
                colony.owner_player_id = Some(format!("capacity-owner-{}", world.colonies.len()));
                world.colonies.push(colony);
            }
        }
        let (mut connection, signed) = authenticated_connection(&state);
        let result = send_action(
            &state,
            &mut connection,
            &ClientAction::FoundVillage {
                name: "One Too Many".to_owned(),
                session_id: signed.session_id,
                sig: Some(signed.sig),
            },
        )
        .await;

        assert!(!result.result.ok);
        assert!(
            result
                .result
                .message
                .as_deref()
                .is_some_and(|message| message.contains("world has reached"))
        );
        assert_eq!(state.world.lock().await.colonies.len(), MAX_TOTAL_COLONIES);
    }

    #[tokio::test]
    async fn one_ip_cannot_found_more_than_the_household_village_allowance() {
        let state = build_state(1_000_000);
        let peer_ip = IpAddr::from([192, 0, 2, 30]);
        let peer = peer_ip.to_string();
        {
            let mut world = state.world.lock().await;
            let template = world.colonies[0].clone();
            let mut guard = state.abuse_guard.lock().await;
            for index in 0..MAX_PERSONAL_VILLAGES_PER_IP {
                let owner = format!("household-owner-{index}");
                let mut colony = template.clone();
                colony.id = format!("household-village-{index}");
                colony.kind = VillageKind::Personal;
                colony.owner_player_id = Some(owner.clone());
                world.colonies.push(colony);
                guard.player_peers.insert(owner, peer.clone());
            }
        }
        let signed = signed_session("household-next".to_owned(), state.session_secret.as_str());
        let mut connection = ConnectionContext::for_peer_ip(peer_ip, STARTER_COLONY_ID.to_owned());
        connection.identity = Some(signed.clone());
        let result = send_action(
            &state,
            &mut connection,
            &ClientAction::FoundVillage {
                name: "Ninth Household Village".to_owned(),
                session_id: signed.session_id,
                sig: Some(signed.sig),
            },
        )
        .await;

        assert!(!result.result.ok);
        assert!(
            result
                .result
                .message
                .as_deref()
                .is_some_and(|message| message.contains("network address"))
        );
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
            entities::{Carrying, CarryingKind, Resources},
            items::{Item, ItemKind, ItemLocation, Material},
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
            pile.contents.stone = 77_234.5;
            pile.contents.weapons = 17_234.5;
            pile.contents.armor = 18_234.5;
            let pile_id = pile.id.clone();
            let equipped_cat_id = colony.cats[0].id.clone();
            let carrier_cat_id = colony.cats[1].id.clone();
            let tool_id = colony
                .items
                .add_at(
                    Item::new(ItemKind::Tool, Material::Metal, 4),
                    1,
                    1.0,
                    ItemLocation::Stockpile {
                        stockpile_id: pile_id.clone(),
                    },
                    true,
                )
                .remove(0);
            let weapon_id = colony
                .items
                .add_at(
                    Item::new(ItemKind::Weapon, Material::Gem, 4),
                    1,
                    1.0,
                    ItemLocation::Equipped {
                        cat_id: equipped_cat_id.clone(),
                    },
                    true,
                )
                .remove(0);
            let armor_id = colony
                .items
                .add_at(
                    Item::new(ItemKind::Armor, Material::Metal, 4),
                    1,
                    1.0,
                    ItemLocation::Carrier {
                        cat_id: carrier_cat_id.clone(),
                    },
                    true,
                )
                .remove(0);
            let mug_id = colony
                .items
                .add_at(
                    Item::new(ItemKind::Mug, Material::Clay, 2),
                    1,
                    1.0,
                    ItemLocation::Stockpile {
                        stockpile_id: pile_id.clone(),
                    },
                    true,
                )
                .remove(0);
            colony.cats[1].carrying = Some(Carrying {
                kind: CarryingKind::Armor,
                amount: 1.0,
                job_ended_at: 1_001_000,
                source_gather_spot: None,
            });
            colony.resources.food = 91_234.5;
            colony.resources.water = 82_234.5;
            colony.resources.stone = 77_234.5;
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
                stone: 5.0,
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
            let canonical_item_ids = canonical.colonies[0]
                .items
                .iter()
                .flat_map(|stack| stack.instances.iter())
                .map(|instance| instance.id.as_str())
                .collect::<BTreeSet<_>>();
            assert!(canonical_item_ids.contains(tool_id.as_str()));
            assert!(canonical_item_ids.contains(weapon_id.as_str()));
            assert!(canonical_item_ids.contains(armor_id.as_str()));
            assert!(canonical_item_ids.contains(mug_id.as_str()));
            assert_eq!(
                canonical.colonies[0]
                    .cats
                    .iter()
                    .find(|cat| cat.id == equipped_cat_id)
                    .expect("canonical equipped cat")
                    .equipment
                    .weapon_item_id
                    .as_deref(),
                Some(weapon_id.as_str())
            );
            assert_eq!(
                canonical.colonies[0]
                    .cats
                    .iter()
                    .find(|cat| cat.id == carrier_cat_id)
                    .expect("canonical carrier cat")
                    .carrying
                    .as_ref()
                    .expect("canonical carrier cargo")
                    .item_ids
                    .as_slice(),
                [armor_id.as_str()]
            );
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
            assert_eq!(player.resources.stone, 5.0);
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
            assert!(
                player
                    .items
                    .iter()
                    .all(|stack| !matches!(stack.kind.as_str(), "tool" | "weapon" | "armor"))
            );
            assert!(
                player
                    .items
                    .iter()
                    .flat_map(|stack| stack.instances.iter())
                    .any(|instance| instance.id == mug_id)
            );
            assert!(
                player
                    .cats
                    .iter()
                    .all(|cat| cat.equipment == cat_protocol::EquipmentLoadoutSnapshot::default())
            );
            assert!(player.cats.iter().all(|cat| {
                cat.carrying
                    .as_ref()
                    .is_none_or(|carrying| carrying.item_ids.is_empty())
            }));

            let json = serde_json::to_value(&projected).expect("owner websocket payload");
            let owner = &json["colonies"][0];
            assert!(owner["stockLedger"].get("accurate").is_none());
            assert!(owner["stockpiles"][0]["report"].get("accurate").is_none());
            let wire = serde_json::to_string(&json).expect("owner websocket JSON");
            for sentinel in [
                "91234.5", "82234.5", "77234.5", "66234.5", "17234.5", "18234.5",
            ] {
                assert!(
                    !wire.contains(sentinel),
                    "authoritative sentinel {sentinel} crossed the player wire"
                );
            }
            for secret_id in [&tool_id, &weapon_id, &armor_id] {
                assert!(
                    !wire.contains(secret_id),
                    "authoritative finite-item id {secret_id} crossed the player wire"
                );
            }
            assert!(wire.contains(&mug_id));
            assert_eq!(canonical.colonies[0].resources.materials, 66_234.5);
            assert_eq!(canonical.colonies[0].stockpiles[0].contents.food, 91_234.5);
        }
    }

    #[test]
    fn exact_equipment_requires_fresh_books_and_the_signed_selected_controller() {
        fn seed_secret_equipment(colony: &mut cat_protocol::ColonySnapshot, id: &str) {
            colony.items.push(cat_protocol::ItemStackSnapshot {
                kind: "tool".to_owned(),
                material: "metal".to_owned(),
                quality: 4,
                count: 1,
                value: 100,
                unit_weight_grams: 1_000,
                instances: vec![cat_protocol::ItemInstanceSnapshot {
                    id: id.to_owned(),
                    durability: 100,
                    max_durability: 100,
                    broken: false,
                    credited: true,
                    location: cat_protocol::ItemLocation::Equipped {
                        cat_id: colony.cats[0].id.clone(),
                    },
                }],
            });
            colony.cats[0].equipment.tool_item_id = Some(id.to_owned());
            colony.cats[0].carrying = Some(cat_protocol::Carrying {
                kind: cat_protocol::CarryingKind::Tools,
                amount: 1.0,
                job_ended_at: 1_001_000,
                item_ids: vec![id.to_owned()],
            });
            colony.trader = Some(cat_protocol::TraderSnapshot {
                id: "trader-fixture".to_owned(),
                position: cat_protocol::TilePoint { x: 0, y: 0 },
                state: cat_protocol::TraderVisitState::Trading,
                destination: None,
                route_exterior: None,
                visit_number: 1,
                arrived_at: Some(1_000_000),
                visit_ends_at: Some(2_000_000),
                coin: 1_000.0,
                cargo_weight_grams: 0.0,
                cargo_capacity_grams: 10_000.0,
                cargo_items: Vec::new(),
                stock: Vec::new(),
                buy_offers: vec![
                    cat_protocol::TraderBuyOffer {
                        kind: "tool".to_owned(),
                        material: "metal".to_owned(),
                        quality: 4,
                        available: 987,
                        unit_price: 100.0,
                        unit_weight_grams: 1_000,
                        blocked_reason: None,
                    },
                    cat_protocol::TraderBuyOffer {
                        kind: "mug".to_owned(),
                        material: "clay".to_owned(),
                        quality: 1,
                        available: 3,
                        unit_price: 4.0,
                        unit_weight_grams: 200,
                        blocked_reason: None,
                    },
                ],
                sell_offers: Vec::new(),
            });
            colony
                .stock_ledger
                .as_mut()
                .expect("canonical stock ledger")
                .accurate = true;
        }

        fn assert_secret_visible(colony: &cat_protocol::ColonySnapshot, id: &str) {
            assert!(
                colony
                    .items
                    .iter()
                    .any(|stack| { stack.instances.iter().any(|instance| instance.id == id) })
            );
            assert_eq!(colony.cats[0].equipment.tool_item_id.as_deref(), Some(id));
            assert_eq!(
                colony.cats[0]
                    .carrying
                    .as_ref()
                    .expect("seeded carrying")
                    .item_ids,
                [id]
            );
            let trader = colony.trader.as_ref().expect("seeded trader");
            assert!(
                trader
                    .buy_offers
                    .iter()
                    .any(|offer| offer.kind == "tool" && offer.available == 987)
            );
        }

        fn assert_secret_redacted(colony: &cat_protocol::ColonySnapshot, id: &str) {
            let wire = serde_json::to_string(colony).expect("projected colony JSON");
            assert!(!wire.contains(id));
            assert!(
                colony
                    .items
                    .iter()
                    .all(|stack| !matches!(stack.kind.as_str(), "tool" | "weapon" | "armor"))
            );
            assert_eq!(
                colony.cats[0].equipment,
                cat_protocol::EquipmentLoadoutSnapshot::default()
            );
            assert!(
                colony.cats[0]
                    .carrying
                    .as_ref()
                    .expect("seeded carrying remains as an aggregate report")
                    .item_ids
                    .is_empty()
            );
            let trader = colony
                .trader
                .as_ref()
                .expect("seeded trader remains visible");
            assert!(
                trader
                    .buy_offers
                    .iter()
                    .all(|offer| !matches!(offer.kind.as_str(), "tool" | "weapon" | "armor"))
            );
            assert!(trader.buy_offers.iter().any(|offer| offer.kind == "mug"));
        }

        let signed = signed_session("exact-equipment-owner".to_owned(), "test-session-secret");
        let mut world = starter_world(1_000_000);
        let mut private = found_colony(
            WORLD_SEED,
            "exact-equipment-private",
            1_000_000,
            STARTER_COLONY_SEED + 1,
        );
        private.kind = VillageKind::Personal;
        private.owner_player_id = Some(signed.player_id.clone());
        world.colonies.push(private);
        let directory = village_directory(&world);
        let mut canonical = build_snapshot(&world, 1_000_000, 1);
        let global_id = "wire-secret-global-tool";
        let private_id = "wire-secret-private-tool";
        seed_secret_equipment(&mut canonical.colonies[0], global_id);
        seed_secret_equipment(&mut canonical.colonies[1], private_id);

        let public = project_snapshot(canonical.clone(), &directory, None, STARTER_COLONY_ID);
        assert_eq!(public.colonies.len(), 1);
        assert_secret_redacted(&public.colonies[0], global_id);

        let selected_private = project_snapshot(
            canonical.clone(),
            &directory,
            Some(&signed),
            "exact-equipment-private",
        );
        assert_eq!(selected_private.colonies[0].id, "exact-equipment-private");
        assert_secret_visible(&selected_private.colonies[0], private_id);
        let unselected_global = selected_private
            .colonies
            .iter()
            .find(|colony| colony.id == STARTER_COLONY_ID)
            .expect("global colony remains visible");
        assert!(unselected_global.capabilities.can_control);
        assert_secret_redacted(unselected_global, global_id);

        let selected_global = project_snapshot(
            canonical.clone(),
            &directory,
            Some(&signed),
            STARTER_COLONY_ID,
        );
        assert_secret_visible(&selected_global.colonies[0], global_id);
        let unselected_private = selected_global
            .colonies
            .iter()
            .find(|colony| colony.id == "exact-equipment-private")
            .expect("owner's personal colony remains visible");
        assert_secret_redacted(unselected_private, private_id);

        canonical.colonies[1]
            .stock_ledger
            .as_mut()
            .expect("private canonical stock ledger")
            .accurate = false;
        let stale_private = project_snapshot(
            canonical,
            &directory,
            Some(&signed),
            "exact-equipment-private",
        );
        assert_secret_redacted(&stale_private.colonies[0], private_id);
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
        private.resources.stone = 68_234.5;
        private.resources.weapons = 17_234.5;
        let storehouse = private
            .stockpiles
            .iter_mut()
            .find(|pile| pile.is_general_storehouse())
            .expect("private storehouse");
        storehouse.contents.food = 91_234.5;
        storehouse.contents.stone = 68_234.5;
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
            assert_eq!(player.resources.stone, 0.0, "{phase} raw Stone report");
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
            for sentinel in ["91234.5", "68234.5", "17234.5", "73456.25"] {
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
        connection.nickname = Some("Test Player".to_owned());
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

    #[test]
    fn every_transport_control_requires_the_standard_signed_session_policy() {
        let common = (
            "session-1".to_owned(),
            "Guest Cat".to_owned(),
            "signed".to_owned(),
        );
        let actions = vec![
            ClientAction::DesignateRail {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                a: TilePoint { x: 1, y: 1 },
                b: TilePoint { x: 4, y: 1 },
                cat_id: "cat-1".to_owned(),
            },
            ClientAction::BuildDock {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                land: TilePoint { x: 1, y: 1 },
                water: TilePoint { x: 1, y: 2 },
                cat_id: "cat-1".to_owned(),
            },
            ClientAction::BuildTransportVehicle {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                mode: cat_protocol::TransportMode::Rail,
                home: TilePoint { x: 1, y: 1 },
                cat_id: "cat-1".to_owned(),
            },
            ClientAction::CreateTransportRoute {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                mode: cat_protocol::TransportMode::Rail,
                source_stockpile_id: "source".to_owned(),
                destination_stockpile_id: "destination".to_owned(),
                resource: cat_protocol::ResourceKind::Food,
                amount: 4.0,
                path: vec![TilePoint { x: 1, y: 1 }, TilePoint { x: 2, y: 1 }],
                cat_id: "cat-1".to_owned(),
                repeat: true,
            },
            ClientAction::CancelTransportRoute {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                route_id: "route-1".to_owned(),
            },
        ];
        for action in &actions {
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
            assert_eq!(colony.stock_ledger.steward_managed_piles.len(), 11);
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
    async fn stale_protocol_client_receives_update_required_before_mutation() {
        let state = build_state(1_000_000);
        let (mut connection, signed) = authenticated_connection(&state);
        let colony_count_before = state.world.lock().await.colonies.len();
        let stale_action = serde_json::json!({
            "action": "foundVillage",
            "protocolVersion": 1,
            "idempotencyId": "stale-found-village",
            "expectedStateVersion": 0,
            "name": "Legacy Hollow",
            "sessionId": signed.session_id,
            "sig": signed.sig,
        });

        let result = handle_client_text(&state, &mut connection, &stale_action.to_string()).await;

        assert!(!result.result.ok, "an incompatible client must fail closed");
        assert!(matches!(
            result.leader_ai,
            Some(LeaderAiServerActionResult::UpdateRequired(_))
        ));
        assert_eq!(
            state.world.lock().await.colonies.len(),
            colony_count_before,
            "version rejection must happen before any mutation"
        );
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
            ClientAction::OfferResource {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                resource: cat_protocol::OfferingResource::Herbs,
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
            let spatial_fixture = colony.clone();
            cat_sim::world_tick::publish_colony_spatial(
                &mut world.shared_spatial,
                &spatial_fixture,
            );
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
                    cat_sim::terrain_gen::resolved_biome_decorations_for_chunks(
                        i64::from(seed),
                        &std::collections::BTreeSet::from([(chunk_x, chunk_y)]),
                    )
                    .into_iter()
                    .find_map(|((x, y), decoration)| {
                        matches!(
                            decoration,
                            cat_sim::terrain_gen::DecorationRole::Tree { .. }
                        )
                        .then(|| cat_sim::terrain_gen::tile_climate_biome(seed, x, y))
                        .filter(|biome| {
                            biome.properties().resource == cat_sim::climate::ResourceHint::Wood
                        })
                        .map(|_| cat_sim::world_tick::TilePos { x, y })
                    })
                })
                .expect("bounded climate scan contains a resolved logging tree");
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
            colony.revealed_tiles.insert(tree);
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
            (
                "guidance-wood-cutter",
                cat_sim::types::BuildingType::WoodCutter,
                56,
            ),
            (
                "guidance-stone-prep",
                cat_sim::types::BuildingType::StonePrep,
                60,
            ),
            (
                "guidance-woodworking",
                cat_sim::types::BuildingType::Woodworking,
                64,
            ),
            ("guidance-smithy", cat_sim::types::BuildingType::Smithy, 68),
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
            (
                "guidance-mill",
                cat_sim::world_tick::FLOUR_TO_FOOD_RECIPE_ID,
            ),
            ("guidance-workshop", cat_sim::world_tick::WORKSHOP_RECIPE_ID),
            ("guidance-smelter", cat_sim::world_tick::SMELTER_RECIPE_ID),
            (
                "guidance-wood-cutter",
                cat_sim::world_tick::WOOD_CUTTER_RECIPE_ID,
            ),
            (
                "guidance-stone-prep",
                cat_sim::station_recipes::STONE_TO_BLOCKS_RECIPE_ID,
            ),
            (
                "guidance-woodworking",
                cat_sim::station_recipes::PLANKS_AND_BLOCKS_TO_TOOLS_RECIPE_ID,
            ),
            (
                "guidance-smithy",
                cat_sim::world_tick::SMITHY_TOOL_RECIPE_ID,
            ),
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
            "smithy",
            "school",
            "advanced_storage",
            "carpentry_sources",
            "carpentry_preparation",
            "grain_milling_sources",
            "grain_milling_preparation",
            "grain_milling_staples",
            "metallurgy_sources",
            "metallurgy_preparation",
            "trade_goods_sources",
            "trade_goods_preparation",
            "toolmaking_sources",
            "toolmaking_preparation",
            "toolmaking_staples",
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
        // Model a second ordinary player burst after the research-tree shopping burst.
        *state.rate_limiter.lock().await =
            RateLimiter::new(ACTION_LIMIT_MAX, ACTION_LIMIT_WINDOW_MS);

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
        let result = send_action(
            &state,
            &mut connection,
            &ClientAction::AssignWorker {
                session_id: signed.session_id.clone(),
                nickname: "Guide".to_owned(),
                sig: signed.sig.clone(),
                cat_id: cat_id.clone(),
                building_id: Some("guidance-woodworking".to_owned()),
            },
        )
        .await;
        assert!(
            result.result.ok,
            "signed Woodworking assignment failed: {result:?}"
        );
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
            if *building_id == "guidance-woodworking" {
                assert_eq!(building.assigned_cat.as_deref(), Some(cat_id.as_str()));
            }
        }
        for study in [
            "carpentry_preparation",
            "grain_milling_preparation",
            "grain_milling_staples",
            "metallurgy_preparation",
            "trade_goods_preparation",
            "toolmaking_staples",
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
    async fn signed_hmac_player_guides_every_physical_woodworking_stage() {
        let started_at = 1_000_000;
        let secret = "guided-woodworking-secret";
        let mut world = starter_world(started_at);
        let colony = &mut world.colonies[0];
        colony.resources.food = 500.0;
        colony.resources.water = 500.0;
        colony.resources.tools = 0.0;
        cat_sim::world_tick::reconcile_colony_stockpiles(colony);
        let woodworking_id = colony
            .buildings
            .iter()
            .find(|building| building.building_type == cat_sim::types::BuildingType::Woodworking)
            .expect("global founding has Woodworking")
            .id
            .clone();
        let cat_id = colony.cats[0].id.clone();
        let conn = Connection::open_in_memory().expect("guided Woodworking database");
        persistence::init_schema(&conn).expect("guided Woodworking schema");
        let state = build_state_from_world(world, conn, secret.to_owned(), false, started_at);
        let signed = signed_session("guided-woodworking-session".to_owned(), secret);
        let mut connection = ConnectionContext::new(
            "guided-woodworking-socket".to_owned(),
            STARTER_COLONY_ID.to_owned(),
        );
        connection.identity = Some(signed.clone());
        let signed_action = |edit| ClientAction::EditProductionQueue {
            session_id: signed.session_id.clone(),
            nickname: "Woodworker".to_owned(),
            sig: signed.sig.clone(),
            building_id: woodworking_id.clone(),
            edit,
        };
        let removed = send_action(
            &state,
            &mut connection,
            &signed_action(cat_protocol::ProductionQueueEdit::Remove { index: 0 }),
        )
        .await;
        assert!(
            removed.result.ok,
            "signed queue removal failed: {removed:?}"
        );
        let queued = send_action(
            &state,
            &mut connection,
            &signed_action(cat_protocol::ProductionQueueEdit::Add {
                recipe_id: cat_sim::station_recipes::PLANKS_AND_BLOCKS_TO_TOOLS_RECIPE_ID
                    .to_owned(),
                repeat: false,
            }),
        )
        .await;
        assert!(queued.result.ok, "signed queue add failed: {queued:?}");
        let assigned = send_action(
            &state,
            &mut connection,
            &ClientAction::AssignWorker {
                session_id: signed.session_id.clone(),
                nickname: "Woodworker".to_owned(),
                sig: signed.sig.clone(),
                cat_id,
                building_id: Some(woodworking_id.clone()),
            },
        )
        .await;
        assert!(assigned.result.ok, "signed assignment failed: {assigned:?}");

        let mut stages = [false; 7];
        for second in 1..=1_200_i64 {
            let mut world = state.world.lock().await;
            let reports = world_tick(&mut world, started_at + second * 1_000);
            assert_eq!(reports[0].reset_reason, None);
            let colony = &world.colonies[0];
            let building = colony
                .buildings
                .iter()
                .find(|building| building.id == woodworking_id)
                .unwrap();
            let inbound = cat_sim::world_tick::building_station_cargo(colony, building, "in");
            let local_input =
                cat_sim::world_tick::building_station_inventory(colony, building, false);
            let local_output =
                cat_sim::world_tick::building_station_inventory(colony, building, true);
            let outbound = cat_sim::world_tick::building_station_cargo(colony, building, "out");
            stages[0] |= inbound.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Planks && *amount == 2.0
            });
            stages[1] |= local_input.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Planks && *amount == 2.0
            });
            stages[2] |= inbound.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Blocks && *amount == 2.0
            });
            stages[3] |= local_input.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Blocks && *amount == 2.0
            });
            stages[4] |= local_output.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Tools && *amount == 1.0
            });
            stages[5] |= outbound.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Tools && *amount == 1.0
            });
            stages[6] |= colony.resources.tools >= 1.0;
            if stages.iter().all(|seen| *seen) {
                break;
            }
        }
        let world = state.world.lock().await;
        let colony = &world.colonies[0];
        assert_eq!(stages, [true; 7], "signed route missed a physical stage");
        assert_eq!(
            colony.items.count_kind(cat_sim::items::ItemKind::Tool),
            1,
            "the delivered scalar Tool must be backed by one exact finite unit"
        );
        let tool = colony
            .items
            .instances()
            .find(|instance| instance.item.kind == cat_sim::items::ItemKind::Tool)
            .expect("guided Woodworking route creates one exact Tool identity");
        assert!(tool.credited, "final delivery credits the exact Tool unit");
        assert!(matches!(
            tool.location,
            cat_sim::items::ItemLocation::Stockpile { .. }
        ));
    }

    #[tokio::test]
    async fn signed_hmac_hunt_and_tannery_route_survives_restart_to_delivered_leather() {
        // `send_action` timestamps authenticated jobs from the server clock, so
        // drive the deterministic tick horizon from the same epoch.
        let started_at = now_ms();
        let secret = "guided-tannery-secret";
        let path = std::env::temp_dir().join(format!(
            "cat-server-tannery-route-{}-{}.db",
            std::process::id(),
            NEXT_DATABASE_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_file(&path);
        let mut world = starter_world(started_at);
        let colony = &mut world.colonies[0];
        colony.resources.food = 100.0;
        colony.resources.water = 100.0;
        colony
            .upgrade_tree
            .owned_node_ids
            .push("textiles".to_owned());
        let anchor = colony.anchor;
        let tannery_id = "signed-guided-tannery".to_owned();
        // A new founding intentionally has no Tannery. This completed bench is an
        // established-colony fixture; its input still comes only from the real Hunt.
        colony.buildings.push(cat_sim::world_tick::BuildingRuntime {
            id: tannery_id.clone(),
            building_type: cat_sim::types::BuildingType::Tannery,
            position: cat_sim::world_tick::TilePos {
                x: anchor.x + 6,
                y: anchor.y + 6,
            },
            is_complete: true,
            construction_progress: 100,
            production_queue: cat_sim::world_tick::default_production_queue(
                cat_sim::types::BuildingType::Tannery,
            ),
            ..cat_sim::world_tick::BuildingRuntime::default()
        });
        cat_sim::world_tick::reconcile_colony_stockpiles(colony);
        let cat_id = colony.cats.last().unwrap().id.clone();
        let conn = Connection::open(&path).expect("guided Tannery database");
        persistence::init_schema(&conn).expect("guided Tannery schema");
        let state = build_state_from_world(world, conn, secret.to_owned(), true, started_at);
        let signed = signed_session("guided-tannery-session".to_owned(), secret);
        let mut connection = ConnectionContext::new(
            "guided-tannery-socket".to_owned(),
            STARTER_COLONY_ID.to_owned(),
        );
        connection.identity = Some(signed.clone());
        let accelerated = send_action(
            &state,
            &mut connection,
            &ClientAction::SetTestAcceleration {
                preset: cat_protocol::AccelerationPreset::Hyper,
            },
        )
        .await;
        assert!(
            accelerated.result.ok,
            "bounded route acceleration: {accelerated:?}"
        );
        let queue_action = |edit| ClientAction::EditProductionQueue {
            session_id: signed.session_id.clone(),
            nickname: "Tanner".to_owned(),
            sig: signed.sig.clone(),
            building_id: tannery_id.clone(),
            edit,
        };
        for edit in [
            cat_protocol::ProductionQueueEdit::Remove { index: 0 },
            cat_protocol::ProductionQueueEdit::Add {
                recipe_id: cat_sim::station_recipes::HIDE_TO_LEATHER_RECIPE_ID.to_owned(),
                repeat: false,
            },
            cat_protocol::ProductionQueueEdit::SetPaused { paused: true },
        ] {
            let result = send_action(&state, &mut connection, &queue_action(edit)).await;
            assert!(result.result.ok, "signed Tannery queue: {result:?}");
        }
        let assigned = send_action(
            &state,
            &mut connection,
            &ClientAction::AssignWorker {
                session_id: signed.session_id.clone(),
                nickname: "Tanner".to_owned(),
                sig: signed.sig.clone(),
                cat_id,
                building_id: Some(tannery_id.clone()),
            },
        )
        .await;
        assert!(
            assigned.result.ok,
            "signed Tannery assignment: {assigned:?}"
        );
        let hunted = send_action(
            &state,
            &mut connection,
            &ClientAction::RequestJob {
                session_id: signed.session_id.clone(),
                nickname: "Tanner".to_owned(),
                sig: signed.sig.clone(),
                kind: cat_protocol::JobKind::HuntExpedition,
            },
        )
        .await;
        assert!(hunted.result.ok, "signed Hunt: {hunted:?}");
        let hunt_job_id = {
            let world = state.world.lock().await;
            world.colonies[0]
                .jobs
                .iter()
                .find(|job| {
                    job.kind == cat_sim::types::JobKind::HuntExpedition
                        && job.requested_by == cat_sim::world_tick::JobRequester::Player
                })
                .expect("signed action created a player-requested Hunt")
                .id
                .clone()
        };

        let mut now = started_at;
        let mut saw_hunt_hide = false;
        let mut saw_delivered_hide = false;
        for second in 1..=2_400_i64 {
            now = started_at + second * 1_000;
            let mut world = state.world.lock().await;
            let reports = world_tick(&mut world, now);
            assert_eq!(reports[0].reset_reason, None);
            let colony = &world.colonies[0];
            let player_hunter = colony
                .jobs
                .iter()
                .find(|job| job.id == hunt_job_id)
                .and_then(|job| job.assigned_cat.as_deref());
            saw_hunt_hide |= player_hunter.is_some_and(|cat_id| {
                colony.cats.iter().any(|cat| {
                    cat.id == cat_id
                        && cat.carrying.as_ref().is_some_and(|cargo| {
                            cargo.kind == cat_sim::entities::CarryingKind::Hide
                                && !cargo
                                    .source_gather_spot
                                    .as_deref()
                                    .is_some_and(|marker| marker.starts_with("station-in|"))
                        })
                })
            });
            saw_delivered_hide |= colony.resources.hide > 0.0;
            let player_hunt_completed = colony.jobs.iter().any(|job| {
                job.id == hunt_job_id
                    && job.requested_by == cat_sim::world_tick::JobRequester::Player
                    && job.status == cat_sim::types::JobStatus::Completed
            });
            if saw_hunt_hide && saw_delivered_hide && player_hunt_completed {
                break;
            }
        }
        let delivered_hide = {
            let world = state.world.lock().await;
            let colony = &world.colonies[0];
            let job = colony
                .jobs
                .iter()
                .find(|job| job.id == hunt_job_id)
                .expect("player Hunt remains persisted");
            let hunter = job
                .assigned_cat
                .as_deref()
                .and_then(|cat_id| colony.cats.iter().find(|cat| cat.id == cat_id));
            assert!(
                saw_hunt_hide && saw_delivered_hide,
                "player Hunt route stalled: saw_hunt_hide={saw_hunt_hide}, saw_delivered_hide={saw_delivered_hide}, job={job:?}, hunter={hunter:?}, colony_hide={}, local_hide={}",
                colony.resources.hide,
                colony
                    .stockpiles
                    .iter()
                    .find(|pile| pile.id == cat_sim::stockpiles::station_input_id(&tannery_id))
                    .map_or(0.0, |pile| pile.contents.hide),
            );
            assert!(world.colonies[0].jobs.iter().any(|job| {
                job.id == hunt_job_id
                    && job.requested_by == cat_sim::world_tick::JobRequester::Player
                    && job.status == cat_sim::types::JobStatus::Completed
            }));
            colony.resources.hide
        };
        save_current_world(&state)
            .await
            .expect("persist delivered Hunt Hide");
        drop(state);

        let conn = Connection::open(&path).expect("reopen Tannery route database");
        persistence::init_schema(&conn).expect("migrate Tannery route database");
        let restarted = build_state_from_connection(now, conn, secret.to_owned())
            .expect("restart delivered Hunt Hide");
        {
            let world = restarted.world.lock().await;
            let colony = &world.colonies[0];
            let tannery = colony
                .buildings
                .iter()
                .find(|building| building.id == tannery_id)
                .expect("restored authored Tannery");
            assert!(tannery.production_paused);
            assert_eq!(tannery.production_progress, 0.0);
            assert_eq!(tannery.production_queue.len(), 1);
            assert_eq!(
                tannery.production_queue[0].recipe_id,
                cat_sim::station_recipes::HIDE_TO_LEATHER_RECIPE_ID
            );
            assert!(!tannery.production_queue[0].repeat);
            assert_eq!(colony.resources.hide, delivered_hide);
            assert_eq!(colony.resources.leather, 0.0);
            assert!(
                cat_sim::world_tick::building_station_inventory(colony, tannery, false).is_empty()
            );
            assert!(
                cat_sim::world_tick::building_station_inventory(colony, tannery, true).is_empty()
            );
            assert!(cat_sim::world_tick::building_station_cargo(colony, tannery, "in").is_empty());
            assert!(cat_sim::world_tick::building_station_cargo(colony, tannery, "out").is_empty());
        }
        let mut reconnected = ConnectionContext::new(
            "guided-tannery-reconnected".to_owned(),
            STARTER_COLONY_ID.to_owned(),
        );
        let presence = send_action(
            &restarted,
            &mut reconnected,
            &ClientAction::Presence {
                session_id: signed.session_id.clone(),
                nickname: "Tanner".to_owned(),
                sig: Some(signed.sig.clone()),
            },
        )
        .await;
        assert!(presence.result.ok, "Tannery bearer reconnect: {presence:?}");
        assert_eq!(
            presence.fields.get("playerId"),
            Some(&signed.player_id),
            "restart must rebind the same stable player identity"
        );
        let resumed = send_action(
            &restarted,
            &mut reconnected,
            &queue_action(cat_protocol::ProductionQueueEdit::SetPaused { paused: false }),
        )
        .await;
        assert!(resumed.result.ok, "signed Tannery resume: {resumed:?}");
        let mut stages = [false; 5];
        for second in 1..=1_800_i64 {
            let mut world = restarted.world.lock().await;
            let reports = world_tick(&mut world, now + second * 1_000);
            assert_eq!(reports[0].reset_reason, None);
            let colony = &world.colonies[0];
            let building = colony
                .buildings
                .iter()
                .find(|building| building.id == tannery_id)
                .expect("restarted Tannery");
            let inbound = cat_sim::world_tick::building_station_cargo(colony, building, "in");
            let local_input =
                cat_sim::world_tick::building_station_inventory(colony, building, false);
            let local_output =
                cat_sim::world_tick::building_station_inventory(colony, building, true);
            let outbound = cat_sim::world_tick::building_station_cargo(colony, building, "out");
            stages[0] |= inbound.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Hide && *amount >= 5.0
            });
            stages[1] |= local_input.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Hide && *amount >= 5.0
            });
            stages[2] |= local_output.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Leather && *amount >= 1.0
            });
            stages[3] |= outbound.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Leather && *amount >= 1.0
            });
            stages[4] |= colony.resources.leather >= 1.0;
            if stages.iter().all(|seen| *seen) {
                break;
            }
        }
        let world = restarted.world.lock().await;
        assert_eq!(stages, [true; 5], "restart missed a physical Tannery stage");
        assert_eq!(world.colonies[0].resources.leather, 1.0);
        drop(world);
        drop(restarted);
        fs::remove_file(path).expect("remove Tannery route database");
    }

    #[tokio::test]
    async fn signed_hmac_fibre_forage_and_clothier_route_survives_restart_to_delivered_cloth() {
        let started_at = now_ms();
        let secret = "guided-clothier-secret";
        let path = std::env::temp_dir().join(format!(
            "cat-server-clothier-route-{}-{}.db",
            std::process::id(),
            NEXT_DATABASE_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_file(&path);
        let mut world = starter_world(started_at);
        let colony = &mut world.colonies[0];
        colony.resources.food = 100.0;
        colony.resources.water = 100.0;
        colony
            .upgrade_tree
            .owned_node_ids
            .push("textiles".to_owned());
        let anchor = colony.anchor;
        let clothier_id = "signed-guided-clothier".to_owned();
        colony.buildings.push(cat_sim::world_tick::BuildingRuntime {
            id: clothier_id.clone(),
            building_type: cat_sim::types::BuildingType::Clothier,
            position: cat_sim::world_tick::TilePos {
                x: anchor.x + 6,
                y: anchor.y + 6,
            },
            is_complete: true,
            construction_progress: 100,
            production_queue: cat_sim::world_tick::default_production_queue(
                cat_sim::types::BuildingType::Clothier,
            ),
            ..cat_sim::world_tick::BuildingRuntime::default()
        });
        cat_sim::world_tick::reconcile_colony_stockpiles(colony);
        let cat_id = colony.cats.last().unwrap().id.clone();
        let conn = Connection::open(&path).expect("guided Clothier database");
        persistence::init_schema(&conn).expect("guided Clothier schema");
        let state = build_state_from_world(world, conn, secret.to_owned(), true, started_at);
        let signed = signed_session("guided-clothier-session".to_owned(), secret);
        let mut connection = ConnectionContext::new(
            "guided-clothier-socket".to_owned(),
            STARTER_COLONY_ID.to_owned(),
        );
        connection.identity = Some(signed.clone());
        assert!(
            send_action(
                &state,
                &mut connection,
                &ClientAction::SetTestAcceleration {
                    preset: cat_protocol::AccelerationPreset::Hyper,
                },
            )
            .await
            .result
            .ok
        );
        let queue_action = |edit| ClientAction::EditProductionQueue {
            session_id: signed.session_id.clone(),
            nickname: "Weaver".to_owned(),
            sig: signed.sig.clone(),
            building_id: clothier_id.clone(),
            edit,
        };
        for edit in [
            cat_protocol::ProductionQueueEdit::Remove { index: 0 },
            cat_protocol::ProductionQueueEdit::Remove { index: 0 },
            cat_protocol::ProductionQueueEdit::Add {
                recipe_id: cat_sim::station_recipes::FIBRE_TO_THREAD_RECIPE_ID.to_owned(),
                repeat: false,
            },
            cat_protocol::ProductionQueueEdit::Add {
                recipe_id: cat_sim::station_recipes::FIBRE_TO_CLOTH_RECIPE_ID.to_owned(),
                repeat: false,
            },
            cat_protocol::ProductionQueueEdit::SetPaused { paused: true },
        ] {
            let result = send_action(&state, &mut connection, &queue_action(edit)).await;
            assert!(result.result.ok, "signed Clothier queue: {result:?}");
        }
        assert!(
            send_action(
                &state,
                &mut connection,
                &ClientAction::AssignWorker {
                    session_id: signed.session_id.clone(),
                    nickname: "Weaver".to_owned(),
                    sig: signed.sig.clone(),
                    cat_id,
                    building_id: Some(clothier_id.clone()),
                },
            )
            .await
            .result
            .ok
        );
        let mut now = started_at;
        let mut saw_raw_fibre = false;
        for request in 0..5 {
            let result = send_action(
                &state,
                &mut connection,
                &ClientAction::RequestJob {
                    session_id: signed.session_id.clone(),
                    nickname: "Weaver".to_owned(),
                    sig: signed.sig.clone(),
                    kind: cat_protocol::JobKind::ForageFibre,
                },
            )
            .await;
            assert!(
                result.result.ok,
                "signed Fibre forage {request}: {result:?}"
            );
            let expected = f64::from(request + 1);
            for _ in 0..600 {
                now += 1_000;
                let mut world = state.world.lock().await;
                let reports = world_tick(&mut world, now);
                assert_eq!(reports[0].reset_reason, None);
                let colony = &world.colonies[0];
                saw_raw_fibre |= colony.cats.iter().any(|cat| {
                    cat.carrying.as_ref().is_some_and(|cargo| {
                        cargo.kind == cat_sim::entities::CarryingKind::Fibre
                            && cargo.source_gather_spot.is_none()
                    })
                });
                if colony.resources.fibre >= expected {
                    break;
                }
            }
            assert!(
                state.world.lock().await.colonies[0].resources.fibre >= expected,
                "signed Fibre forage {request} did not return to storage"
            );
        }
        {
            let world = state.world.lock().await;
            assert!(saw_raw_fibre, "signed forage never placed Fibre in paws");
            assert!(world.colonies[0].resources.fibre >= 5.0);
        }
        save_current_world(&state)
            .await
            .expect("persist delivered Fibre");
        drop(state);

        let conn = Connection::open(&path).expect("reopen Clothier route database");
        persistence::init_schema(&conn).expect("migrate Clothier route database");
        let restarted = build_state_from_connection(now, conn, secret.to_owned())
            .expect("restart delivered Fibre");
        {
            let world = restarted.world.lock().await;
            let colony = &world.colonies[0];
            let clothier = colony
                .buildings
                .iter()
                .find(|building| building.id == clothier_id)
                .expect("restored authored Clothier");
            assert!(clothier.production_paused);
            assert_eq!(clothier.production_queue.len(), 2);
            assert_eq!(
                clothier.production_queue[0].recipe_id,
                cat_sim::station_recipes::FIBRE_TO_THREAD_RECIPE_ID
            );
            assert_eq!(
                clothier.production_queue[1].recipe_id,
                cat_sim::station_recipes::FIBRE_TO_CLOTH_RECIPE_ID
            );
            assert!(clothier.production_queue.iter().all(|entry| !entry.repeat));
            assert_eq!(colony.resources.cloth, 0.0);
            assert!(colony.resources.fibre >= 5.0);
        }
        let mut reconnected = ConnectionContext::new(
            "guided-clothier-reconnected".to_owned(),
            STARTER_COLONY_ID.to_owned(),
        );
        let presence = send_action(
            &restarted,
            &mut reconnected,
            &ClientAction::Presence {
                session_id: signed.session_id.clone(),
                nickname: "Weaver".to_owned(),
                sig: Some(signed.sig.clone()),
            },
        )
        .await;
        assert!(
            presence.result.ok,
            "Clothier bearer reconnect: {presence:?}"
        );
        assert_eq!(presence.fields.get("playerId"), Some(&signed.player_id));
        assert!(
            send_action(
                &restarted,
                &mut reconnected,
                &queue_action(cat_protocol::ProductionQueueEdit::SetPaused { paused: false }),
            )
            .await
            .result
            .ok
        );
        let mut stages = [false; 9];
        for second in 1..=1_800_i64 {
            let mut world = restarted.world.lock().await;
            let reports = world_tick(&mut world, now + second * 1_000);
            assert_eq!(reports[0].reset_reason, None);
            let colony = &world.colonies[0];
            let building = colony
                .buildings
                .iter()
                .find(|building| building.id == clothier_id)
                .unwrap();
            let inbound = cat_sim::world_tick::building_station_cargo(colony, building, "in");
            let local_input =
                cat_sim::world_tick::building_station_inventory(colony, building, false);
            let local_output =
                cat_sim::world_tick::building_station_inventory(colony, building, true);
            let outbound = cat_sim::world_tick::building_station_cargo(colony, building, "out");
            stages[0] |= inbound.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Fibre && *amount >= 5.0
            });
            stages[1] |= local_input.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Fibre && *amount >= 5.0
            });
            stages[2] |= local_output.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Thread && *amount >= 5.0
            });
            stages[3] |= outbound.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Thread && *amount >= 5.0
            });
            stages[4] |= colony.resources.thread >= 5.0;
            stages[5] |= inbound.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Thread && *amount >= 5.0
            }) || local_input.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Thread && *amount >= 5.0
            });
            stages[6] |= local_output.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Cloth && *amount >= 1.0
            });
            stages[7] |= outbound.iter().any(|(kind, amount)| {
                *kind == cat_sim::stockpiles::ResourceKind::Cloth && *amount >= 1.0
            });
            stages[8] |= colony.resources.cloth >= 1.0;
            if stages.iter().all(|seen| *seen) {
                break;
            }
        }
        let world = restarted.world.lock().await;
        assert_eq!(
            stages, [true; 9],
            "restart missed a physical Clothier stage"
        );
        assert_eq!(world.colonies[0].resources.cloth, 1.0);
        drop(world);
        drop(restarted);
        fs::remove_file(path).expect("remove Clothier route database");
    }

    #[tokio::test]
    async fn signed_hmac_ore_smelter_smithy_route_survives_sqlite_restart() {
        let started_at = now_ms();
        let secret = "guided-smithy-secret";
        let path = std::env::temp_dir().join(format!(
            "cat-server-smithy-route-{}-{}.db",
            std::process::id(),
            NEXT_DATABASE_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_file(&path);
        let mut world = starter_world(started_at);
        let colony = &mut world.colonies[0];
        colony.resources.food = 100.0;
        colony.resources.water = 100.0;
        colony.resources.ore = 10.0;
        colony
            .upgrade_tree
            .owned_node_ids
            .extend(["metallurgy_preparation", "weaponsmithing"].map(str::to_owned));
        let anchor = colony.anchor;
        for (id, building_type, offset) in [
            (
                "signed-guided-smelter",
                cat_sim::types::BuildingType::Smelter,
                6,
            ),
            (
                "signed-guided-smithy",
                cat_sim::types::BuildingType::Smithy,
                10,
            ),
        ] {
            colony.buildings.push(cat_sim::world_tick::BuildingRuntime {
                id: id.to_owned(),
                building_type,
                position: cat_sim::world_tick::TilePos {
                    x: anchor.x + offset,
                    y: anchor.y + 6,
                },
                is_complete: true,
                construction_progress: 100,
                production_queue: cat_sim::world_tick::default_production_queue(building_type),
                ..cat_sim::world_tick::BuildingRuntime::default()
            });
        }
        cat_sim::world_tick::reconcile_colony_stockpiles(colony);
        let worker_ids = [
            colony.cats[colony.cats.len() - 2].id.clone(),
            colony.cats[colony.cats.len() - 1].id.clone(),
        ];
        let conn = Connection::open(&path).expect("guided Smithy database");
        persistence::init_schema(&conn).expect("guided Smithy schema");
        let state = build_state_from_world(world, conn, secret.to_owned(), true, started_at);
        let signed = signed_session("guided-smithy-session".to_owned(), secret);
        let mut connection = ConnectionContext::new(
            "guided-smithy-socket".to_owned(),
            STARTER_COLONY_ID.to_owned(),
        );
        connection.identity = Some(signed.clone());
        assert!(
            send_action(
                &state,
                &mut connection,
                &ClientAction::SetTestAcceleration {
                    preset: cat_protocol::AccelerationPreset::Hyper,
                },
            )
            .await
            .result
            .ok
        );
        let queue_action = |building_id: &str, edit| ClientAction::EditProductionQueue {
            session_id: signed.session_id.clone(),
            nickname: "Smith".to_owned(),
            sig: signed.sig.clone(),
            building_id: building_id.to_owned(),
            edit,
        };
        for (building_id, removals, recipe_id, repeat) in [
            (
                "signed-guided-smelter",
                1_usize,
                cat_sim::station_recipes::SMELTER_RECIPE_ID,
                true,
            ),
            (
                "signed-guided-smithy",
                3_usize,
                cat_sim::station_recipes::SMITHY_WEAPON_RECIPE_ID,
                false,
            ),
        ] {
            for _ in 0..removals {
                let result = send_action(
                    &state,
                    &mut connection,
                    &queue_action(
                        building_id,
                        cat_protocol::ProductionQueueEdit::Remove { index: 0 },
                    ),
                )
                .await;
                assert!(result.result.ok, "signed queue removal: {result:?}");
            }
            for edit in [
                cat_protocol::ProductionQueueEdit::Add {
                    recipe_id: recipe_id.to_owned(),
                    repeat,
                },
                cat_protocol::ProductionQueueEdit::SetPaused { paused: true },
            ] {
                let result =
                    send_action(&state, &mut connection, &queue_action(building_id, edit)).await;
                assert!(result.result.ok, "signed queue authoring: {result:?}");
            }
        }
        for (cat_id, building_id) in worker_ids
            .iter()
            .zip(["signed-guided-smelter", "signed-guided-smithy"])
        {
            let result = send_action(
                &state,
                &mut connection,
                &ClientAction::AssignWorker {
                    session_id: signed.session_id.clone(),
                    nickname: "Smith".to_owned(),
                    sig: signed.sig.clone(),
                    cat_id: cat_id.clone(),
                    building_id: Some(building_id.to_owned()),
                },
            )
            .await;
            assert!(result.result.ok, "signed worker assignment: {result:?}");
        }
        save_current_world(&state)
            .await
            .expect("persist paused Smithy chain");
        drop(state);

        let conn = Connection::open(&path).expect("reopen Smithy route database");
        persistence::init_schema(&conn).expect("migrate Smithy route database");
        let restarted = build_state_from_connection(started_at, conn, secret.to_owned())
            .expect("restart paused Smithy chain");
        {
            let world = restarted.world.lock().await;
            for id in ["signed-guided-smelter", "signed-guided-smithy"] {
                let building = world.colonies[0]
                    .buildings
                    .iter()
                    .find(|building| building.id == id)
                    .unwrap();
                assert!(building.production_paused);
                assert_eq!(building.production_queue.len(), 1);
                assert_eq!(building.production_progress, 0.0);
            }
        }
        let mut reconnected = ConnectionContext::new(
            "guided-smithy-reconnected".to_owned(),
            STARTER_COLONY_ID.to_owned(),
        );
        let presence = send_action(
            &restarted,
            &mut reconnected,
            &ClientAction::Presence {
                session_id: signed.session_id.clone(),
                nickname: "Smith".to_owned(),
                sig: Some(signed.sig.clone()),
            },
        )
        .await;
        assert!(presence.result.ok, "Smithy bearer reconnect: {presence:?}");
        assert_eq!(presence.fields.get("playerId"), Some(&signed.player_id));
        for id in ["signed-guided-smelter", "signed-guided-smithy"] {
            assert!(
                send_action(
                    &restarted,
                    &mut reconnected,
                    &queue_action(
                        id,
                        cat_protocol::ProductionQueueEdit::SetPaused { paused: false },
                    ),
                )
                .await
                .result
                .ok
            );
        }

        let mut stages = [false; 9];
        for second in 1..=3_600_i64 {
            let mut world = restarted.world.lock().await;
            let reports = world_tick(&mut world, started_at + second * 1_000);
            assert_eq!(reports[0].reset_reason, None);
            let colony = &world.colonies[0];
            let smelter = colony
                .buildings
                .iter()
                .find(|building| building.id == "signed-guided-smelter")
                .unwrap();
            let smithy = colony
                .buildings
                .iter()
                .find(|building| building.id == "signed-guided-smithy")
                .unwrap();
            let has = |building, direction, kind| {
                cat_sim::world_tick::building_station_cargo(colony, building, direction)
                    .iter()
                    .any(|(found, amount)| *found == kind && *amount > 0.0)
            };
            let local = |building, output, kind| {
                cat_sim::world_tick::building_station_inventory(colony, building, output)
                    .iter()
                    .any(|(found, amount)| *found == kind && *amount > 0.0)
            };
            stages[0] |= has(smelter, "in", cat_sim::stockpiles::ResourceKind::Ore);
            stages[1] |= local(smelter, false, cat_sim::stockpiles::ResourceKind::Ore);
            stages[2] |= local(smelter, true, cat_sim::stockpiles::ResourceKind::Metal);
            stages[3] |= has(smelter, "out", cat_sim::stockpiles::ResourceKind::Metal);
            stages[4] |= has(smithy, "in", cat_sim::stockpiles::ResourceKind::Metal);
            stages[5] |= local(smithy, false, cat_sim::stockpiles::ResourceKind::Metal);
            stages[6] |= local(smithy, true, cat_sim::stockpiles::ResourceKind::Weapons);
            stages[7] |= has(smithy, "out", cat_sim::stockpiles::ResourceKind::Weapons);
            stages[8] |= colony.resources.weapons >= 1.0;
            if stages.iter().all(|seen| *seen) {
                break;
            }
        }
        let world = restarted.world.lock().await;
        assert_eq!(stages, [true; 9], "restart missed a physical Smithy stage");
        assert_eq!(world.colonies[0].resources.weapons, 1.0);
        assert_eq!(world.colonies[0].resources.armor, 0.0);
        assert_eq!(
            world.colonies[0]
                .items
                .count_kind(cat_sim::items::ItemKind::Weapon),
            1,
            "the delivered scalar Weapon must be backed by one exact finite unit"
        );
        assert_eq!(
            world.colonies[0]
                .items
                .count_kind(cat_sim::items::ItemKind::Armor),
            0
        );
        let weapon = world.colonies[0]
            .items
            .instances()
            .find(|instance| instance.item.kind == cat_sim::items::ItemKind::Weapon)
            .expect("guided Smithy route creates one exact Weapon identity");
        assert!(
            weapon.credited,
            "final delivery credits the exact Weapon unit"
        );
        assert!(matches!(
            weapon.location,
            cat_sim::items::ItemLocation::Stockpile { .. }
        ));
        drop(world);
        drop(restarted);
        fs::remove_file(path).expect("remove Smithy route database");
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
            route_blocked: false,
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
    async fn signed_exact_equipment_bearer_and_sqlite_restart_preserve_one_identity() {
        use cat_sim::items::{Item, ItemKind, ItemLocation, Material};

        let path = std::env::temp_dir().join(format!(
            "cat-server-finite-equipment-restart-{}-{}.db",
            std::process::id(),
            NEXT_DATABASE_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_file(&path);
        let secret = "finite-equipment-restart-secret";
        let signed = signed_session("equipment-owner".to_owned(), secret);
        let intruder = signed_session("equipment-intruder".to_owned(), secret);
        let mut world = starter_world(1_000_000);
        let cat_id = world.colonies[0].cats[0].id.clone();
        let dead_cat_id = world.colonies[0].cats[1].id.clone();
        let other_cat_id = world.colonies[0].cats[2].id.clone();
        world.colonies[0].cats[1].death_time = Some(999_000);
        let item = Item::new(ItemKind::Weapon, Material::Metal, 1);
        let item_id = world.colonies[0]
            .items
            .add_at(item, 1, 1.0, ItemLocation::LegacyTreasury, true)
            .pop()
            .expect("one finite weapon");
        let mut foreign = found_colony(WORLD_SEED, "foreign-equipment", 1_000_000, 404);
        foreign.kind = VillageKind::Personal;
        foreign.owner_player_id = Some(intruder.player_id.clone());
        foreign.cats[0].id = "foreign-equipment-cat".to_owned();
        let foreign_cat_id = foreign.cats[0].id.clone();
        let foreign_item_id = foreign
            .items
            .add_at(item, 2, 1.0, ItemLocation::LegacyTreasury, true)
            .pop()
            .expect("foreign finite weapon");
        world.colonies.push(foreign);

        let conn = Connection::open(&path).expect("open equipment database");
        persistence::init_schema(&conn).expect("init equipment database");
        let state = build_state_from_world(world, conn, secret.to_owned(), false, 1_000_000);
        let mut connection =
            ConnectionContext::new("equipment-socket".to_owned(), STARTER_COLONY_ID.to_owned());
        connection.identity = Some(signed.clone());
        let equip = |cat_id: String, item_id: String, session_id: String, sig: String| {
            ClientAction::EquipItem {
                session_id,
                nickname: "Equipment Guide".to_owned(),
                sig,
                cat_id,
                item_id,
            }
        };

        let forged = send_action(
            &state,
            &mut connection,
            &equip(
                cat_id.clone(),
                item_id.clone(),
                signed.session_id.clone(),
                "forged".to_owned(),
            ),
        )
        .await;
        assert!(!forged.result.ok, "forged HMAC equipped an item");
        let foreign_bearer = send_action(
            &state,
            &mut connection,
            &equip(
                cat_id.clone(),
                item_id.clone(),
                intruder.session_id.clone(),
                intruder.sig.clone(),
            ),
        )
        .await;
        assert!(
            !foreign_bearer.result.ok,
            "a valid but different bearer controlled this socket"
        );
        let unknown_item = send_action(
            &state,
            &mut connection,
            &equip(
                cat_id.clone(),
                "item-unknown".to_owned(),
                signed.session_id.clone(),
                signed.sig.clone(),
            ),
        )
        .await;
        assert!(!unknown_item.result.ok, "unknown item equipped");
        let foreign_item = send_action(
            &state,
            &mut connection,
            &equip(
                cat_id.clone(),
                foreign_item_id,
                signed.session_id.clone(),
                signed.sig.clone(),
            ),
        )
        .await;
        assert!(
            !foreign_item.result.ok,
            "an item from an unselected foreign colony crossed authority"
        );
        let foreign_cat = send_action(
            &state,
            &mut connection,
            &equip(
                foreign_cat_id,
                item_id.clone(),
                signed.session_id.clone(),
                signed.sig.clone(),
            ),
        )
        .await;
        assert!(
            !foreign_cat.result.ok,
            "a cat from an unselected foreign colony crossed authority"
        );
        let dead_bearer = send_action(
            &state,
            &mut connection,
            &equip(
                dead_cat_id.clone(),
                item_id.clone(),
                signed.session_id.clone(),
                signed.sig.clone(),
            ),
        )
        .await;
        assert!(!dead_bearer.result.ok, "dead cat equipped an item");

        let equipped = send_action(
            &state,
            &mut connection,
            &equip(
                cat_id.clone(),
                item_id.clone(),
                signed.session_id.clone(),
                signed.sig.clone(),
            ),
        )
        .await;
        assert!(equipped.result.ok, "signed equip failed: {equipped:?}");
        {
            let world = state.world.lock().await;
            assert_eq!(
                world.colonies[0]
                    .items
                    .instance(&item_id)
                    .expect("same identity")
                    .location,
                ItemLocation::Equipped {
                    cat_id: cat_id.clone()
                }
            );
        }

        let wrong_cat = send_action(
            &state,
            &mut connection,
            &ClientAction::UnequipItem {
                session_id: signed.session_id.clone(),
                nickname: "Equipment Guide".to_owned(),
                sig: signed.sig.clone(),
                cat_id: other_cat_id,
                item_id: item_id.clone(),
            },
        )
        .await;
        assert!(
            !wrong_cat.result.ok,
            "a different bearer unequipped the exact item"
        );

        save_current_world(&state)
            .await
            .expect("persist equipped identity");
        drop(state);

        let conn = Connection::open(&path).expect("reopen equipment database");
        persistence::init_schema(&conn).expect("migrate equipment database");
        let restarted = build_state_from_connection(2_000_000, conn, secret.to_owned())
            .expect("restore equipment world");
        {
            let world = restarted.world.lock().await;
            let instance = world.colonies[0]
                .items
                .instance(&item_id)
                .expect("same identity after restart");
            assert_eq!(
                instance.location,
                ItemLocation::Equipped {
                    cat_id: cat_id.clone()
                }
            );
            assert!(instance.credited);
            let snapshot = build_snapshot(&world, 2_000_000, 1);
            let cat = snapshot.colonies[0]
                .cats
                .iter()
                .find(|cat| cat.id == cat_id)
                .expect("equipped cat snapshot");
            assert_eq!(
                cat.equipment.weapon_item_id.as_deref(),
                Some(item_id.as_str())
            );
        }

        let mut restarted_connection = ConnectionContext::new(
            "equipment-socket-restarted".to_owned(),
            STARTER_COLONY_ID.to_owned(),
        );
        restarted_connection.identity = Some(signed.clone());
        let unequipped = send_action(
            &restarted,
            &mut restarted_connection,
            &ClientAction::UnequipItem {
                session_id: signed.session_id,
                nickname: "Equipment Guide".to_owned(),
                sig: signed.sig,
                cat_id: cat_id.clone(),
                item_id: item_id.clone(),
            },
        )
        .await;
        assert!(
            unequipped.result.ok,
            "signed unequip failed: {unequipped:?}"
        );
        let world = restarted.world.lock().await;
        assert_ne!(
            world.colonies[0]
                .items
                .instance(&item_id)
                .expect("identity survives unequip")
                .location,
            ItemLocation::Equipped { cat_id }
        );
        drop(world);
        drop(restarted);
        fs::remove_file(path).expect("remove equipment database");
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
    async fn signed_selectable_offerings_persist_the_chosen_physical_resource() {
        use cat_protocol::OfferingResource;
        use cat_sim::{
            entities::{CatActivity, Resources},
            stockpiles::ResourceKind as SimResourceKind,
            types::JobKind as SimJobKind,
            world_tick::{JobMetadata, reconcile_colony_stockpiles},
        };

        let mut world = new_world(WORLD_SEED);
        let mut colony = found_colony(
            WORLD_SEED,
            STARTER_COLONY_ID,
            1_000_000,
            STARTER_COLONY_SEED,
        );
        colony.resources = Resources {
            food: 200.0,
            herbs: 50.0,
            materials: 100.0,
            ..colony.resources
        };
        reconcile_colony_stockpiles(&mut colony);
        world.colonies.push(colony);
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        persistence::init_schema(&conn).expect("init in-memory schema");
        persistence::save_world(&conn, &world).expect("seed signed offering world");
        let state = build_state_from_world(
            world,
            conn,
            "test-session-secret".to_owned(),
            false,
            1_000_000,
        );
        let (mut connection, signed) = authenticated_connection(&state);

        for (resource, expected) in [
            (OfferingResource::Food, SimResourceKind::Food),
            (OfferingResource::Herbs, SimResourceKind::Herbs),
            (OfferingResource::Materials, SimResourceKind::Materials),
        ] {
            let action = ClientAction::OfferResource {
                session_id: signed.session_id.clone(),
                nickname: "Shrine Guide".to_owned(),
                sig: signed.sig.clone(),
                resource,
            };
            let result = send_action(&state, &mut connection, &action).await;
            assert!(
                result.result.ok,
                "signed {resource:?} offering failed: {result:?}"
            );
            save_current_world(&state)
                .await
                .expect("persist signed selectable offering");

            let persisted = {
                let db = state.db.lock().await;
                persistence::load_world(&db)
                    .expect("load signed offering")
                    .expect("world persisted")
            };
            assert!(persisted.colonies[0].jobs.iter().any(|job| {
                job.kind == SimJobKind::CarryOffering
                    && matches!(
                        job.metadata,
                        JobMetadata::OfferingCarry { kind, .. } if kind == expected
                    )
            }));

            let mut live = state.world.lock().await;
            live.colonies[0].jobs.clear();
            for cat in &mut live.colonies[0].cats {
                cat.current_task = None;
                cat.activity = CatActivity::Idle;
                cat.destination = None;
                cat.carrying = None;
            }
        }
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
    async fn two_signed_players_found_discover_and_physically_trade_between_villages() {
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

            // This integration test verifies signed offer/acceptance and the physical caravan
            // lifecycle, not whether these two hashed player sites happen to share a naturally
            // passable wilderness corridor for this world seed. Materialize one deterministic
            // meadow trail through both real gates so acceptance still exercises the production
            // route planner and its wall-edge validation without depending on incidental rivers.
            let gate_pair = |colony: &cat_sim::world_tick::ColonyRuntime| {
                let area = cat_sim::village_area::from_tiles(
                    &colony
                        .claimed_tiles
                        .iter()
                        .map(|tile| cat_sim::village_layout::GridPos {
                            x: tile.x,
                            y: tile.y,
                        })
                        .collect::<Vec<_>>(),
                );
                let gate = cat_sim::village_area::gate_placement_default(&area)
                    .expect("founded village gate");
                let delta = cat_sim::village_area::side_delta(gate.side);
                (
                    TilePos {
                        x: gate.x,
                        y: gate.y,
                    },
                    TilePos {
                        x: gate.x + delta.x,
                        y: gate.y + delta.y,
                    },
                )
            };
            let (first_gate, first_outside) = gate_pair(&world.colonies[first_index]);
            let (second_gate, second_outside) = gate_pair(&world.colonies[second_index]);
            let corridor_y = first_outside.y.max(second_outside.y) + 64;
            let waypoints = [
                first_gate,
                first_outside,
                TilePos {
                    x: first_outside.x,
                    y: corridor_y,
                },
                TilePos {
                    x: second_outside.x,
                    y: corridor_y,
                },
                second_outside,
                second_gate,
            ];
            for pair in waypoints.windows(2) {
                let mut pos = pair[0];
                loop {
                    world.shared_spatial.tiles.insert(
                        pos,
                        cat_sim::world_tick::WorldTileRuntime {
                            pos,
                            tile_type: cat_sim::types::TileType::Meadow,
                            resources: cat_sim::world_gen::TileResources {
                                food: 0,
                                herbs: 0,
                                water: 0,
                                gem: 0,
                                clay: 0,
                                sand: 0,
                            },
                            max_resources: cat_sim::biomes::MaxResources { food: 0, herbs: 0 },
                            danger_level: 0.0,
                            path_wear: 0,
                            last_depleted: 0,
                            overlay_feature: None,
                        },
                    );
                    if pos == pair[1] {
                        break;
                    }
                    pos.x += (pair[1].x - pos.x).signum();
                    pos.y += (pair[1].y - pos.y).signum();
                }
            }
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

        let mut world = state.world.lock().await;
        let first_before_arrival = world
            .colonies
            .iter()
            .find(|colony| colony.id == first_id)
            .expect("first colony");
        let second_before_arrival = world
            .colonies
            .iter()
            .find(|colony| colony.id == second_id)
            .expect("second colony");
        assert_eq!(first_before_arrival.resources.food, 68.0);
        assert_eq!(second_before_arrival.resources.materials, 73.0);
        assert!(first_before_arrival.resources.materials < 67.0);
        assert!(second_before_arrival.resources.food < 62.0);
        assert_eq!(first_before_arrival.village_trade_caravans.len(), 1);
        let accepted_at = first_before_arrival
            .village_trade_caravans
            .values()
            .next()
            .expect("physical caravan")
            .accepted_at;

        cat_sim::actions::advance_village_trade_caravans(&mut world, accepted_at + 86_400_000);
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

    async fn assert_renewed_owner_retains_village(old: SignedSession) {
        let private_id = "renewal-private-village";
        let mut world = starter_world(1_000_000);
        let mut personal = found_colony(WORLD_SEED, private_id, 1_000_000, 77);
        personal.kind = VillageKind::Personal;
        personal.owner_player_id = Some(old.player_id.clone());
        world.colonies.push(personal);
        let state = build_test_state_from_world(world, 1_000_000);
        let mut connection =
            ConnectionContext::new("renewal-browser".to_owned(), STARTER_COLONY_ID.to_owned());
        let presence = send_action(
            &state,
            &mut connection,
            &ClientAction::Presence {
                session_id: old.session_id.clone(),
                nickname: "Returning Owner".to_owned(),
                sig: Some(old.sig.clone()),
            },
        )
        .await;
        assert!(presence.result.ok, "renewal presence: {presence:?}");
        assert_eq!(presence.fields.get("playerId"), Some(&old.player_id));
        let renewed_session = presence.fields["sessionId"].clone();
        let renewed_sig = presence.fields["sig"].clone();
        assert_ne!(renewed_session, old.session_id);

        let joined = send_action(
            &state,
            &mut connection,
            &ClientAction::JoinVillage {
                colony_id: private_id.to_owned(),
                session_id: renewed_session,
                sig: Some(renewed_sig),
            },
        )
        .await;
        assert!(joined.result.ok, "renewed owner joins village: {joined:?}");
        let snapshot = current_snapshot(&state, 1, &connection).await;
        assert_eq!(snapshot.colonies[0].id, private_id);
        assert!(snapshot.colonies[0].capabilities.is_owner);
    }

    async fn assert_unrenewable_credential_does_not_inherit_village(
        old: SignedSession,
        supplied_sig: String,
    ) {
        let private_id = "rejected-renewal-private-village";
        let mut world = starter_world(1_000_000);
        let mut personal = found_colony(WORLD_SEED, private_id, 1_000_000, 78);
        personal.kind = VillageKind::Personal;
        personal.owner_player_id = Some(old.player_id.clone());
        world.colonies.push(personal);
        let state = build_test_state_from_world(world, 1_000_000);
        let mut connection = ConnectionContext::new(
            "rejected-renewal-browser".to_owned(),
            STARTER_COLONY_ID.to_owned(),
        );
        let presence = send_action(
            &state,
            &mut connection,
            &ClientAction::Presence {
                session_id: old.session_id,
                nickname: "Untrusted Return".to_owned(),
                sig: Some(supplied_sig),
            },
        )
        .await;
        assert!(
            presence.result.ok,
            "fresh replacement presence: {presence:?}"
        );
        assert_ne!(presence.fields.get("playerId"), Some(&old.player_id));

        let joined = send_action(
            &state,
            &mut connection,
            &ClientAction::JoinVillage {
                colony_id: private_id.to_owned(),
                session_id: presence.fields["sessionId"].clone(),
                sig: Some(presence.fields["sig"].clone()),
            },
        )
        .await;
        assert!(
            !joined.result.ok,
            "replacement identity cannot inherit owner"
        );
    }

    #[tokio::test]
    async fn legacy_owner_session_upgrades_without_stranding_personal_village() {
        let old = signed_session(
            "session_0123456789abcdef0123456789abcdef".to_owned(),
            "guided-campaign-secret",
        );
        assert_renewed_owner_retains_village(old).await;
    }

    #[tokio::test]
    async fn recently_expired_v1_owner_renews_without_stranding_personal_village() {
        let issued_at = now_ms() - identity::SESSION_MAX_AGE_MS - 1;
        let old = signed_session(
            format!("session_v1_{issued_at}_0123456789abcdef0123456789abcdef"),
            "guided-campaign-secret",
        );
        assert_renewed_owner_retains_village(old).await;
    }

    #[tokio::test]
    async fn tampered_and_over_grace_credentials_cannot_inherit_personal_village() {
        let issued_at = now_ms() - identity::SESSION_MAX_AGE_MS - 1;
        let tampered = signed_session(
            format!("session_v1_{issued_at}_11111111111111111111111111111111"),
            "guided-campaign-secret",
        );
        assert_unrenewable_credential_does_not_inherit_village(
            tampered,
            "tampered-signature".to_owned(),
        )
        .await;

        let too_old_at =
            now_ms() - identity::SESSION_MAX_AGE_MS - identity::SESSION_RENEWAL_GRACE_MS - 1;
        let too_old = signed_session(
            format!("session_v1_{too_old_at}_22222222222222222222222222222222"),
            "guided-campaign-secret",
        );
        let authentic_sig = too_old.sig.clone();
        assert_unrenewable_credential_does_not_inherit_village(too_old, authentic_sig).await;
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

    fn lai27_envelope(
        signed: &SignedSession,
        colony_id: &str,
        idempotency_id: &str,
        versions: CurrentVersionHint,
    ) -> LeaderAiActionEnvelope {
        LeaderAiActionEnvelope {
            protocol_version: ActionProtocolVersion::current(),
            idempotency_id: cat_protocol::ActionIdempotencyId::new(idempotency_id)
                .expect("valid action id"),
            colony_id: cat_protocol::SelectedColonyId::new(colony_id)
                .expect("valid selected colony"),
            player_id: cat_protocol::AuthenticatedPlayerId::new(signed.player_id.clone())
                .expect("valid player id"),
            expected_versions: cat_protocol::ExpectedStateVersions {
                expected_planner_version: versions.planner_version.expect("planner version"),
                expected_domain_version: versions.domain_version.expect("domain version"),
                expected_resource_version: versions.resource_version.expect("resource version"),
                expected_spatial_version: versions.spatial_version,
                expected_reservation_version: versions.reservation_version,
                expected_research_version: versions.research_version,
                expected_scholar_version: versions.scholar_version,
                expected_boost_version: versions.boost_version,
                expected_diplomacy_version: versions.diplomacy_version,
                expected_trade_version: versions.trade_version,
                expected_prosthetic_version: versions.prosthetic_version,
                expected_care_version: versions.care_version,
                expected_officer_version: versions.officer_version,
                expected_standing_order_version: versions.standing_order_version,
            },
            payload: cat_protocol::LeaderAiActionPayload::NudgePlan {
                plan_id: BoundedEntityId::new("plan:visible").expect("valid plan id"),
                nudge: cat_protocol::BoundedBasisPointNudge::new(1_500)
                    .expect("valid bounded nudge"),
                reason_key: None,
            },
        }
    }

    #[tokio::test]
    async fn typed_start_lifecycle_founds_selects_and_never_uses_legacy_gameplay_frames() {
        let state = build_state(1_000_000);
        let (mut connection, signed) = authenticated_connection(&state);
        let global_versions = {
            let world = state.world.lock().await;
            current_server_state_versions(&world, STARTER_COLONY_ID).expect("global versions")
        };
        let mut found = lai27_envelope(
            &signed,
            STARTER_COLONY_ID,
            "typed-start-found",
            global_versions,
        );
        found.payload = cat_protocol::LeaderAiActionPayload::FoundVillage {
            display_name: cat_protocol::BoundedVillageName::new("Typed Hollow")
                .expect("bounded village name"),
        };

        let founded = handle_client_text(
            &state,
            &mut connection,
            &serde_json::to_string(&found).expect("serialize typed foundation"),
        )
        .await;
        assert!(matches!(
            founded.leader_ai,
            Some(LeaderAiServerActionResult::Action(response))
                if matches!(response.result, LeaderAiActionResult::Accepted { .. })
        ));
        let personal_id = connection.colony_id.clone();
        assert_ne!(personal_id, STARTER_COLONY_ID);
        {
            let world = state.world.lock().await;
            let personal = world
                .colonies
                .iter()
                .find(|colony| colony.id == personal_id)
                .expect("typed action founded a personal village");
            assert_eq!(personal.name, "Typed Hollow");
            assert_eq!(
                personal.owner_player_id.as_deref(),
                Some(signed.player_id.as_str())
            );
        }

        let personal_versions = {
            let world = state.world.lock().await;
            current_server_state_versions(&world, &personal_id).expect("personal versions")
        };
        let mut select = lai27_envelope(
            &signed,
            &personal_id,
            "typed-start-select-global",
            personal_versions,
        );
        select.payload = cat_protocol::LeaderAiActionPayload::SelectColony {
            target_colony_id: cat_protocol::SelectedColonyId::new(STARTER_COLONY_ID)
                .expect("global colony id"),
        };
        let selected = handle_client_text(
            &state,
            &mut connection,
            &serde_json::to_string(&select).expect("serialize typed selection"),
        )
        .await;
        assert!(matches!(
            selected.leader_ai,
            Some(LeaderAiServerActionResult::Action(response))
                if matches!(response.result, LeaderAiActionResult::Accepted { .. })
        ));
        assert_eq!(connection.colony_id, STARTER_COLONY_ID);
    }

    #[tokio::test]
    async fn lai27_live_route_orders_stale_then_receipted_rejection_and_duplicate_replay() {
        let state = build_state(1_000_000);
        let (mut connection, signed) = authenticated_connection(&state);
        let current = {
            let world = state.world.lock().await;
            current_server_state_versions(&world, STARTER_COLONY_ID).expect("current versions")
        };
        let mut stale = lai27_envelope(&signed, STARTER_COLONY_ID, "lai27:stale", current.clone());
        stale.expected_versions.expected_planner_version += 1;
        let stale_result = handle_client_text(
            &state,
            &mut connection,
            &serde_json::to_string(&stale).expect("serialize stale"),
        )
        .await;
        assert!(matches!(
            stale_result.leader_ai,
            Some(LeaderAiServerActionResult::Action(response))
                if matches!(
                    response.result,
                    LeaderAiActionResult::Rejected {
                        conflict: cat_protocol::ActionConflict::VersionMismatch { .. }
                    }
                )
        ));
        assert!(state.leader_ai_receipts.lock().await.is_empty());

        let action = lai27_envelope(&signed, STARTER_COLONY_ID, "lai27:replay", current);
        let encoded = serde_json::to_string(&action).expect("serialize action");
        let rejected = handle_client_text(&state, &mut connection, &encoded).await;
        assert!(matches!(
            rejected.leader_ai,
            Some(LeaderAiServerActionResult::Action(response))
                if matches!(
                    response.result,
                    LeaderAiActionResult::Rejected {
                        conflict: cat_protocol::ActionConflict::PreconditionFailed { .. }
                    }
                )
        ));
        assert_eq!(state.leader_ai_receipts.lock().await.len(), 1);

        let replayed = handle_client_text(&state, &mut connection, &encoded).await;
        assert!(matches!(
            replayed.leader_ai,
            Some(LeaderAiServerActionResult::Action(response))
                if matches!(
                    response.result,
                    LeaderAiActionResult::DuplicateReplay { .. }
                )
        ));

        let persisted_world = state.world.lock().await.clone();
        let restarted_db = Connection::open_in_memory().expect("open restart sqlite");
        persistence::init_schema(&restarted_db).expect("initialize restart schema");
        let restarted = build_state_from_world(
            persisted_world,
            restarted_db,
            state.session_secret.as_str().to_owned(),
            false,
            1_000_001,
        );
        let (mut restarted_connection, _) = authenticated_connection(&restarted);
        let restart_replay =
            handle_client_text(&restarted, &mut restarted_connection, &encoded).await;
        assert!(matches!(
            restart_replay.leader_ai,
            Some(LeaderAiServerActionResult::Action(response))
                if matches!(
                    response.result,
                    LeaderAiActionResult::DuplicateReplay { .. }
                )
        ));
    }

    #[tokio::test]
    async fn lai27_old_client_and_foreign_colony_fail_before_world_mutation() {
        let state = build_state(1_000_000);
        let mut unauthenticated =
            ConnectionContext::new("old-client".to_owned(), STARTER_COLONY_ID.to_owned());
        let before = state.world.lock().await.clone();
        let update = handle_client_text(
            &state,
            &mut unauthenticated,
            r#"{"protocolVersion":1,"payload":{"action":"future_secret_action"}}"#,
        )
        .await;
        assert!(matches!(
            update.leader_ai,
            Some(LeaderAiServerActionResult::UpdateRequired(_))
        ));
        let malformed = handle_client_text(
            &state,
            &mut unauthenticated,
            r#"{"protocolVersion":"current","payload":{"action":"future_secret_action"}}"#,
        )
        .await;
        assert!(matches!(
            malformed.leader_ai,
            Some(LeaderAiServerActionResult::ProtocolError(conflict))
                if matches!(*conflict, cat_protocol::ActionConflict::MalformedPayload)
        ));
        assert_eq!(*state.world.lock().await, before);

        let (mut connection, signed) = authenticated_connection(&state);
        let refreshed_directory = {
            let mut world = state.world.lock().await;
            let mut foreign = world.colonies[0].clone();
            foreign.id = "colony:foreign-private".to_owned();
            foreign.kind = VillageKind::Personal;
            foreign.owner_player_id = Some("different-player".to_owned());
            world.colonies.push(foreign);
            village_directory(&world)
        };
        *state.village_directory.write().await = refreshed_directory;
        let current = {
            let world = state.world.lock().await;
            current_server_state_versions(&world, "colony:foreign-private")
                .expect("foreign current versions")
        };
        let action = lai27_envelope(&signed, "colony:foreign-private", "lai27:foreign", current);
        let denied = handle_client_text(
            &state,
            &mut connection,
            &serde_json::to_string(&action).expect("serialize foreign action"),
        )
        .await;
        assert!(matches!(
            denied.leader_ai,
            Some(LeaderAiServerActionResult::Action(response))
                if matches!(
                    response.result,
                    LeaderAiActionResult::Rejected {
                        conflict: cat_protocol::ActionConflict::OwnershipDenied
                    }
                )
        ));
        assert!(state.leader_ai_receipts.lock().await.is_empty());
    }

    #[tokio::test]
    async fn lai27d_standing_order_commits_once_and_replays_without_second_mutation() {
        let state = build_state(1_000_000);
        let (mut connection, signed) = authenticated_connection(&state);
        let current = {
            let world = state.world.lock().await;
            current_server_state_versions(&world, STARTER_COLONY_ID).expect("current versions")
        };
        let before_snapshot = {
            let world = state.world.lock().await;
            let directory = state.village_directory.read().await;
            build_report_safe_leader_ai_snapshot(&world, &directory, &connection, 1_000_000)
                .expect("pre-action snapshot")
        };
        assert_eq!(
            before_snapshot.colonies[0].action_versions, current,
            "LAI.24 must publish the exact version vector accepted by LAI.25"
        );
        let mut action =
            lai27_envelope(&signed, STARTER_COLONY_ID, "lai27d:standing-order", current);
        action.payload = cat_protocol::LeaderAiActionPayload::CreateStandingOrder {
            order_kind: BoundedEntityId::new("reserve_floor").unwrap(),
            domain: BoundedEntityId::new("forestry").unwrap(),
            target_id: None,
            instruction: cat_protocol::BoundedStandingOrderText::new(
                "Maintain a visible lumber reserve.",
            )
            .unwrap(),
            priority_basis_points: cat_protocol::BoundedBasisPoints::new(8_000).unwrap(),
            expires_at_ms: None,
        };
        let encoded = serde_json::to_string(&action).unwrap();
        let accepted = handle_client_text(&state, &mut connection, &encoded).await;
        assert!(matches!(
            accepted.leader_ai,
            Some(LeaderAiServerActionResult::Action(response))
                if matches!(response.result, LeaderAiActionResult::Accepted { .. })
        ));
        let committed = state.world.lock().await.colonies[0]
            .leader_ai_runtime
            .player_directives
            .clone();
        assert_eq!(committed.standing_orders.len(), 1);
        assert_eq!(committed.version, 1);
        let after_snapshot = {
            let world = state.world.lock().await;
            let directory = state.village_directory.read().await;
            build_report_safe_leader_ai_snapshot(&world, &directory, &connection, 1_000_001)
                .expect("post-action snapshot")
        };
        assert_ne!(
            before_snapshot.colonies[0].state_version,
            after_snapshot.colonies[0].state_version
        );
        let encoded_snapshot = serde_json::to_string(&after_snapshot).unwrap();
        assert_eq!(
            LeaderAiSnapshotEnvelope::decode_json(&encoded_snapshot).unwrap(),
            after_snapshot
        );

        let replay = handle_client_text(&state, &mut connection, &encoded).await;
        assert!(matches!(
            replay.leader_ai,
            Some(LeaderAiServerActionResult::Action(response))
                if matches!(response.result, LeaderAiActionResult::DuplicateReplay { .. })
        ));
        assert_eq!(
            state.world.lock().await.colonies[0]
                .leader_ai_runtime
                .player_directives,
            committed
        );
    }

    #[test]
    fn lai27d_current_payload_match_has_no_generic_unsupported_fallback() {
        let production = include_str!("main.rs");
        let retired_fallback = ["action", "not", "available"].join("_");
        assert!(!production.contains(&retired_fallback));
        for variant in [
            "NudgePlan",
            "CreateStandingOrder",
            "UpdateStandingOrder",
            "DeleteStandingOrder",
            "DismissIntent",
            "AppointOfficer",
            "UnappointOfficer",
            "OfficerAuthorityOverride",
            "RequestTreatment",
            "FitProsthetic",
            "RepairProsthetic",
            "PurchaseResearchWithFavor",
            "PrepareScholarStudy",
            "ActivateDivineBoost",
            "ChangeDiplomacy",
            "ApproveAlliance",
            "BlockColony",
            "AcceptTradeContract",
            "RejectTradeContract",
            "PhysicalPlacement",
        ] {
            assert!(
                production.contains(&format!("LeaderAiActionPayload::{variant}")),
                "missing canonical mutation arm for {variant}"
            );
        }
    }

    #[test]
    fn legacy_gameplay_actions_require_lai_v2_except_bootstrap_and_village_lifecycle() {
        let common = (
            "session-1".to_owned(),
            "Guest Cat".to_owned(),
            "signed".to_owned(),
        );
        let retired = [
            ClientAction::PurchaseUpgrade {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                key: cat_protocol::UpgradeKey::ClickPower,
            },
            ClientAction::UnlockNode {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                node_id: "old-blessing-node".to_owned(),
            },
            ClientAction::ResearchNode {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                node_id: "old-research-node".to_owned(),
            },
            ClientAction::OfferTithe {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
            },
            ClientAction::OfferMaterials {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
            },
            ClientAction::OfferResource {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                resource: cat_protocol::OfferingResource::Herbs,
            },
            ClientAction::BoostCat {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                cat_id: "cat-1".to_owned(),
                boosted: true,
            },
            ClientAction::AssignOfficer {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                role: OfficerRole::Steward,
                cat_id: "cat-1".to_owned(),
            },
            ClientAction::UnassignOfficer {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                role: OfficerRole::Steward,
            },
        ];
        assert!(retired.iter().all(legacy_action_requires_lai_v2));

        // The direct controls the canonical boundary supersedes. These were
        // once routed on to `apply_action`; the retirement gate now refuses
        // them, so they belong on the retired side of the classification.
        let superseded_direct_controls = [
            ClientAction::RequestJob {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                kind: cat_protocol::JobKind::SupplyFood,
            },
            ClientAction::Boost {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                job_id: "job-1".to_owned(),
            },
            ClientAction::PlanBuilding {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                building_type: cat_protocol::BuildingType::Den,
                site: None,
            },
            ClientAction::AssignWorker {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                cat_id: "cat-1".to_owned(),
                building_id: None,
            },
            ClientAction::CreateZone {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                kind: cat_protocol::ZoneKind::Gather,
                a: cat_protocol::TilePoint { x: 1, y: 1 },
                b: cat_protocol::TilePoint { x: 2, y: 2 },
                duration_ms: 60_000,
            },
            ClientAction::BuildRoad {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                a: cat_protocol::TilePoint { x: 1, y: 1 },
                b: cat_protocol::TilePoint { x: 3, y: 1 },
            },
            ClientAction::CastVote {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                election_id: "election-1".to_owned(),
                cat_id: "cat-1".to_owned(),
            },
            ClientAction::RequestVoteKick {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
            },
            ClientAction::DesignateFarm {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                a: cat_protocol::TilePoint { x: 1, y: 1 },
                b: cat_protocol::TilePoint { x: 2, y: 2 },
                crop: cat_protocol::CropKind::Grain,
            },
            ClientAction::DesignateStockpile {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                a: cat_protocol::TilePoint { x: 1, y: 1 },
                b: cat_protocol::TilePoint { x: 2, y: 2 },
                accepts: vec![cat_protocol::ResourceKind::Food],
            },
            ClientAction::SetCatLaborPreference {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                cat_id: "cat-1".to_owned(),
                labor: cat_protocol::Labor::Haul,
                enabled: true,
            },
            ClientAction::BuyResource {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                resource: cat_protocol::ResourceKind::Food,
                amount: 1.0,
            },
            ClientAction::TrainWarrior {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: common.2.clone(),
                cat_id: None,
            },
        ];
        assert!(
            superseded_direct_controls
                .iter()
                .all(legacy_action_requires_lai_v2)
        );

        // Exactly the four bootstrap/lifecycle allowances stay on the legacy
        // shell envelope. Nothing else may be added to this list.
        let bootstrap_allowances = [
            ClientAction::Presence {
                session_id: common.0.clone(),
                nickname: common.1.clone(),
                sig: Some(common.2.clone()),
            },
            ClientAction::Ensure,
            ClientAction::FoundVillage {
                name: "Beta".to_owned(),
                session_id: common.0.clone(),
                sig: Some(common.2.clone()),
            },
            ClientAction::JoinVillage {
                colony_id: "colony-1".to_owned(),
                session_id: common.0.clone(),
                sig: Some(common.2.clone()),
            },
        ];
        assert!(
            bootstrap_allowances
                .iter()
                .all(|action| !legacy_action_requires_lai_v2(action))
        );
    }

    #[test]
    fn lai27f_production_route_retires_superseded_legacy_actions_before_apply_action() {
        let production = include_str!("main.rs");
        let socket_start = production.find("async fn handle_socket").unwrap();
        let socket_end = production[socket_start..]
            .find("async fn send_current_snapshot")
            .map(|offset| socket_start + offset)
            .unwrap();
        let socket = &production[socket_start..socket_end];
        assert!(!socket.contains("cfg!(test)"));
        assert!(socket.contains("#[cfg(test)]"));
        assert!(socket.contains("send_leader_ai_snapshot"));

        // The production entry point is `handle_client_text`; the canonical
        // lane is reached through `handle_canonical_action_text`, not a second
        // legacy-named handler. Names are rebuilt from parts so this assertion
        // does not match itself inside `include_str!`.
        let retired_handler = ["handle", "leader", "ai", "client", "text"].join("_");
        assert!(!production.contains(&retired_handler));
        let action_start = production.find("async fn handle_client_text").unwrap();
        let action_end = production[action_start..]
            .find("struct LeaderAiMutationRateLimit")
            .map(|offset| action_start + offset)
            .unwrap();
        let action = &production[action_start..action_end];
        assert!(!action.contains("cfg!(test)"));
        let canonical_dispatch = action.find("handle_canonical_action_text(").unwrap();
        let legacy_decode = action.find("from_str::<ClientAction>").unwrap();
        let retired_gate = action
            .find("if legacy_action_requires_lai_v2(&action)")
            .unwrap();
        let legacy_apply = action
            .find("apply_action(&mut world, &action, &ctx)")
            .unwrap();
        assert!(canonical_dispatch < legacy_decode);
        assert!(legacy_decode < retired_gate);
        assert!(retired_gate < legacy_apply);
        assert!(!action.contains("legacy_action_tag"));

        // Exactly one classification of the legacy union, with no wildcard arm
        // and a single allowance group, so a superseded control can never fall
        // through to `apply_action`.
        let classifier_signature = format!(
            "fn legacy_action_requires_lai_v2(action: &{}) -> bool",
            "ClientAction"
        );
        assert_eq!(production.matches(&classifier_signature).count(), 1);
        let classifier_start = production.find(&classifier_signature).unwrap();
        let classifier_end = production[classifier_start..]
            .find("async fn handle_client_text")
            .map(|offset| classifier_start + offset)
            .unwrap();
        let classifier = &production[classifier_start..classifier_end];
        assert!(!classifier.contains("_ =>"));
        assert_eq!(classifier.matches("=> false").count(), 1);
        let retired_shell_gate = ["legacy", "shell", "action", "allowed"].join("_");
        assert!(!production.contains(&retired_shell_gate));
    }

    #[test]
    fn player_names_are_normalized_and_action_tags_are_readable() {
        assert_eq!(
            normalized_player_name("  Mara   Moos "),
            Ok(Some("Mara Moos".to_owned()))
        );
        assert_eq!(normalized_player_name("   "), Ok(None));
        assert!(normalized_player_name("x").is_err());
        assert_eq!(humanize_camel_case("planBuilding"), "Plan building");
    }
}
