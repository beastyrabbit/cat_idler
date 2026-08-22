//! Test-only real-WebSocket harness for deterministic whole-game journeys.

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use cat_protocol::{ActionResult, ClientAction, WorldSnapshot};
use cat_sim::world_tick::{WorldState, world_tick};
use futures_util::{SinkExt, StreamExt};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;
use tempfile::TempDir;
use tokio::{net::TcpStream, sync::oneshot, task::JoinHandle, time::timeout};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

use crate::{
    AppState, ServerConfig, app, build_state_from_connection, build_state_from_world, hosting,
    persistence, run_tick_once, save_current_world, starter_world,
};

const START_MS: i64 = 1_700_000_000_000;
const SOCKET_TIMEOUT: Duration = Duration::from_secs(10);
const SESSION_SECRET: &str = "deterministic-playtest-session-secret";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SignedActor {
    pub(crate) session_id: String,
    pub(crate) nickname: String,
    pub(crate) sig: String,
    pub(crate) player_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ObservedActionResult {
    pub(crate) result: ActionResult,
    pub(crate) session_id: Option<String>,
    pub(crate) sig: Option<String>,
    pub(crate) player_id: Option<String>,
    pub(crate) raw: Value,
}

impl ObservedActionResult {
    pub(crate) fn actor(&self, nickname: impl Into<String>) -> Option<SignedActor> {
        Some(SignedActor {
            session_id: self.session_id.clone()?,
            nickname: nickname.into(),
            sig: self.sig.clone()?,
            player_id: self.player_id.clone()?,
        })
    }
}

pub(crate) struct WsClient {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    latest_snapshot: WorldSnapshot,
    pub(crate) action_results: Vec<ObservedActionResult>,
}

impl WsClient {
    pub(crate) fn snapshot(&self) -> &WorldSnapshot {
        &self.latest_snapshot
    }

    pub(crate) async fn authenticate(
        &mut self,
        session_id: impl Into<String>,
        nickname: impl Into<String>,
        sig: Option<String>,
    ) -> Result<SignedActor, String> {
        let session_id = session_id.into();
        let nickname = nickname.into();
        let observed = self
            .send_action(&ClientAction::Presence {
                session_id,
                nickname: nickname.clone(),
                sig,
            })
            .await?;
        if !observed.result.ok {
            return Err(format!("presence rejected: {:?}", observed.result.message));
        }
        let actor = observed
            .actor(nickname)
            .ok_or_else(|| "presence result omitted signed identity fields".to_owned())?;
        Ok(actor)
    }

    pub(crate) async fn send_action(
        &mut self,
        action: &ClientAction,
    ) -> Result<ObservedActionResult, String> {
        let text = serde_json::to_string(action).map_err(|error| error.to_string())?;
        self.send_text(text).await
    }

    pub(crate) async fn send_raw(
        &mut self,
        text: impl Into<String>,
    ) -> Result<ObservedActionResult, String> {
        self.send_text(text.into()).await
    }

    async fn send_text(&mut self, text: String) -> Result<ObservedActionResult, String> {
        self.socket
            .send(Message::Text(text.into()))
            .await
            .map_err(|error| format!("send WebSocket action: {error}"))?;
        loop {
            let value = self.receive_value().await?;
            if value.get("colonies").is_some() {
                self.latest_snapshot = serde_json::from_value(value)
                    .map_err(|error| format!("decode projected snapshot: {error}"))?;
                continue;
            }
            if value.get("ok").is_some() {
                let result = serde_json::from_value::<ActionResult>(value.clone())
                    .map_err(|error| format!("decode action result: {error}"))?;
                let observed = ObservedActionResult {
                    result,
                    session_id: value
                        .get("sessionId")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    sig: value.get("sig").and_then(Value::as_str).map(str::to_owned),
                    player_id: value
                        .get("playerId")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    raw: value,
                };
                self.action_results.push(observed.clone());
                return Ok(observed);
            }
            return Err(format!("unrecognized server message: {value}"));
        }
    }

    pub(crate) async fn receive_snapshot(&mut self) -> Result<WorldSnapshot, String> {
        loop {
            let value = self.receive_value().await?;
            if value.get("colonies").is_some() {
                let snapshot = serde_json::from_value::<WorldSnapshot>(value)
                    .map_err(|error| format!("decode projected snapshot: {error}"))?;
                self.latest_snapshot = snapshot.clone();
                return Ok(snapshot);
            }
            if value.get("ok").is_some() {
                let result = serde_json::from_value::<ActionResult>(value.clone())
                    .map_err(|error| format!("decode asynchronous action result: {error}"))?;
                self.action_results.push(ObservedActionResult {
                    result,
                    session_id: value
                        .get("sessionId")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    sig: value.get("sig").and_then(Value::as_str).map(str::to_owned),
                    player_id: value
                        .get("playerId")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    raw: value,
                });
                continue;
            }
            return Err(format!("unrecognized server message: {value}"));
        }
    }

    async fn receive_snapshot_at_or_after(
        &mut self,
        expected_now: i64,
    ) -> Result<WorldSnapshot, String> {
        loop {
            let snapshot = self.receive_snapshot().await?;
            if snapshot.now >= expected_now {
                return Ok(snapshot);
            }
        }
    }

    async fn receive_value(&mut self) -> Result<Value, String> {
        let message = timeout(SOCKET_TIMEOUT, self.socket.next())
            .await
            .map_err(|_| "timed out waiting for WebSocket message".to_owned())?
            .ok_or_else(|| "WebSocket closed".to_owned())?
            .map_err(|error| format!("receive WebSocket message: {error}"))?;
        match message {
            Message::Text(text) => serde_json::from_str(text.as_str())
                .map_err(|error| format!("decode server JSON: {error}")),
            Message::Ping(payload) => {
                self.socket
                    .send(Message::Pong(payload))
                    .await
                    .map_err(|error| format!("reply to WebSocket ping: {error}"))?;
                Box::pin(self.receive_value()).await
            }
            Message::Close(frame) => Err(format!("WebSocket closed: {frame:?}")),
            other => Err(format!("unexpected WebSocket message: {other:?}")),
        }
    }

    pub(crate) async fn close(mut self) {
        let _ = self.socket.close(None).await;
    }
}

pub(crate) struct WsGameHarness {
    _temp_dir: TempDir,
    database_path: PathBuf,
    state: Option<AppState>,
    address: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    server: Option<JoinHandle<Result<(), std::io::Error>>>,
    now_ms: i64,
    tick: u64,
    pub(crate) seed: u32,
}

impl WsGameHarness {
    pub(crate) async fn start(seed: u32) -> Result<Self, String> {
        Self::start_with(seed, |_| {}).await
    }

    pub(crate) async fn start_with(
        seed: u32,
        setup: impl FnOnce(&mut WorldState),
    ) -> Result<Self, String> {
        let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let database_path = temp_dir.path().join("playtest.sqlite3");
        let connection = Connection::open(&database_path).map_err(|error| error.to_string())?;
        persistence::init_schema(&connection).map_err(|error| error.to_string())?;
        let mut world = starter_world(START_MS);
        world.colonies[0].test_rng_seed = Some(seed);
        setup(&mut world);
        let mut state = build_state_from_world(
            world,
            connection,
            SESSION_SECRET.to_owned(),
            false,
            START_MS,
        );
        state.action_now_ms = Some(std::sync::Arc::new(std::sync::atomic::AtomicI64::new(
            START_MS,
        )));
        let (address, shutdown, server) = Self::spawn_server(state.clone()).await?;
        Ok(Self {
            _temp_dir: temp_dir,
            database_path,
            state: Some(state),
            address,
            shutdown: Some(shutdown),
            server: Some(server),
            now_ms: START_MS,
            tick: 0,
            seed,
        })
    }

    async fn spawn_server(
        state: AppState,
    ) -> Result<
        (
            SocketAddr,
            oneshot::Sender<()>,
            JoinHandle<Result<(), std::io::Error>>,
        ),
        String,
    > {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|error| format!("bind playtest server: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("read playtest address: {error}"))?;
        let config = ServerConfig {
            listen_addr: address,
            web_dist: None,
            public_images: None,
            allowed_origins: hosting::AllowedOrigins::default(),
            trusted_proxies: hosting::TrustedProxies::default(),
        };
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                app(state, &config).into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
        });
        Ok((address, shutdown_tx, server))
    }

    pub(crate) fn now_ms(&self) -> i64 {
        self.now_ms
    }

    pub(crate) async fn connect(&self) -> Result<WsClient, String> {
        let url = format!("ws://{}/ws", self.address);
        let (socket, _) = connect_async(url)
            .await
            .map_err(|error| format!("connect to playtest server: {error}"))?;
        let mut client = WsClient {
            socket,
            latest_snapshot: empty_snapshot(),
            action_results: Vec::new(),
        };
        client.receive_snapshot().await?;
        Ok(client)
    }

    pub(crate) async fn connect_authenticated(
        &mut self,
        session_id: impl Into<String>,
        nickname: impl Into<String>,
    ) -> Result<(WsClient, SignedActor), String> {
        let session_id = session_id.into();
        let nickname = nickname.into();
        let mut client = self.connect().await?;
        let actor = client.authenticate(session_id, nickname, None).await?;
        self.broadcast_current_snapshot(&mut client).await?;
        Ok((client, actor))
    }

    async fn broadcast_current_snapshot(&self, client: &mut WsClient) -> Result<(), String> {
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| "playtest server is stopped".to_owned())?
            .clone();
        run_tick_once(state, self.tick, self.now_ms, |_, _| {})
            .await
            .map_err(|error| format!("broadcast deterministic snapshot: {error}"))?;
        client.receive_snapshot().await?;
        Ok(())
    }

    pub(crate) async fn advance_by(
        &mut self,
        client: &mut WsClient,
        delta_ms: i64,
    ) -> Result<WorldSnapshot, String> {
        if delta_ms <= 0 {
            return Err("deterministic time must advance by a positive duration".to_owned());
        }
        self.now_ms = self
            .now_ms
            .checked_add(delta_ms)
            .ok_or_else(|| "deterministic time overflow".to_owned())?;
        self.tick = self.tick.saturating_add(1);
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| "playtest server is stopped".to_owned())?
            .clone();
        state
            .action_now_ms
            .as_ref()
            .expect("playtest harness installs its deterministic action clock")
            .store(self.now_ms, std::sync::atomic::Ordering::SeqCst);
        run_tick_once(state, self.tick, self.now_ms, |world, tick_now| {
            let _ = world_tick(world, tick_now);
        })
        .await
        .map_err(|error| format!("run deterministic authoritative tick: {error}"))?;
        // A successful action broadcasts a same-time projection after its action
        // result. If the caller has not consumed that projection yet, it is ahead
        // of this tick in the socket queue. Never let `advance_by` report that stale
        // action snapshot as evidence for the deterministic tick just executed.
        client.receive_snapshot_at_or_after(self.now_ms).await
    }

    pub(crate) async fn eventually<F>(
        &mut self,
        client: &mut WsClient,
        horizon_ms: i64,
        cadence_ms: i64,
        mut predicate: F,
    ) -> Result<WorldSnapshot, String>
    where
        F: FnMut(&WorldSnapshot) -> bool,
    {
        let deadline = self
            .now_ms
            .checked_add(horizon_ms)
            .ok_or_else(|| "scenario horizon overflow".to_owned())?;
        if predicate(client.snapshot()) {
            return Ok(client.snapshot().clone());
        }
        while self.now_ms < deadline {
            let step = cadence_ms.min(deadline - self.now_ms);
            let snapshot = self.advance_by(client, step).await?;
            if predicate(&snapshot) {
                return Ok(snapshot);
            }
        }
        Err(format!(
            "milestone not reached by deterministic deadline {deadline} (now={})",
            self.now_ms
        ))
    }

    pub(crate) async fn save(&self) -> Result<(), String> {
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| "playtest server is stopped".to_owned())?;
        save_current_world(state)
            .await
            .map_err(|error| format!("save playtest world: {error}"))
    }

    pub(crate) async fn restart_and_reconnect(
        &mut self,
        client: WsClient,
        actor: &SignedActor,
    ) -> Result<WsClient, String> {
        let mut action_results = client.action_results.clone();
        client.close().await;
        self.save().await?;
        self.stop_server().await?;
        self.state.take();

        let connection =
            Connection::open(&self.database_path).map_err(|error| error.to_string())?;
        persistence::init_schema(&connection).map_err(|error| error.to_string())?;
        let mut state =
            build_state_from_connection(self.now_ms, connection, SESSION_SECRET.to_owned())
                .map_err(|error| format!("reload playtest world: {error}"))?;
        state.action_now_ms = Some(std::sync::Arc::new(std::sync::atomic::AtomicI64::new(
            self.now_ms,
        )));
        let (address, shutdown, server) = Self::spawn_server(state.clone()).await?;
        self.state = Some(state);
        self.address = address;
        self.shutdown = Some(shutdown);
        self.server = Some(server);

        let mut reconnected = self.connect().await?;
        let restored_actor = reconnected
            .authenticate(
                actor.session_id.clone(),
                actor.nickname.clone(),
                Some(actor.sig.clone()),
            )
            .await?;
        action_results.append(&mut reconnected.action_results);
        reconnected.action_results = action_results;
        self.broadcast_current_snapshot(&mut reconnected).await?;
        if restored_actor.player_id != actor.player_id {
            return Err("reconnect changed the stable player identity".to_owned());
        }
        Ok(reconnected)
    }

    async fn stop_server(&mut self) -> Result<(), String> {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(server) = self.server.take() {
            server
                .await
                .map_err(|error| format!("join playtest server: {error}"))?
                .map_err(|error| format!("stop playtest server: {error}"))?;
        }
        Ok(())
    }
}

impl Drop for WsGameHarness {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(server) = self.server.take() {
            server.abort();
        }
    }
}

fn empty_snapshot() -> WorldSnapshot {
    WorldSnapshot {
        protocol_version: cat_protocol::PROTOCOL_VERSION,
        now: START_MS,
        world_seed: 0,
        colonies: Vec::new(),
        online_count: 0,
        selected_colony_id: None,
        known_villages: Vec::new(),
        village_trade_offers: Vec::new(),
        village_trade_caravans: Vec::new(),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FailureTrace<'a> {
    pub(crate) scenario_id: &'a str,
    pub(crate) seed: u32,
    pub(crate) last_completed_milestone: Option<&'a str>,
    pub(crate) simulated_time_ms: i64,
    pub(crate) action_results: &'a [ObservedActionResult],
    pub(crate) snapshot: &'a WorldSnapshot,
    pub(crate) restart_difference: Option<&'a Value>,
    pub(crate) failure: &'a str,
}

pub(crate) fn write_failure_trace(trace: &FailureTrace<'_>) -> Result<PathBuf, String> {
    let requested = std::env::var_os("CAT_PLAYTEST_TRACE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/playtest-traces"));
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("cat-server must remain under the workspace crates directory");
    let directory = if requested.is_absolute() {
        requested
    } else {
        workspace_root.join(requested)
    };
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("create playtest trace directory: {error}"))?;
    let path = directory.join(format!("{}-{}.json", trace.scenario_id, trace.seed));
    write_pretty_json(&path, trace)?;
    Ok(path)
}

fn write_pretty_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("serialize playtest trace: {error}"))?;
    std::fs::write(path, bytes).map_err(|error| format!("write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn real_socket_auth_tick_save_restart_and_reconnect_is_deterministic() {
        let mut harness = WsGameHarness::start(4_242).await.expect("start harness");
        let (mut client, actor) = harness
            .connect_authenticated("playtest-install", "Playtest Cat")
            .await
            .expect("authenticate over WebSocket");
        assert_eq!(client.snapshot().colonies[0].id, "colony-1");

        let before = client.snapshot().now;
        let queued = client
            .send_action(&ClientAction::RequestJob {
                session_id: actor.session_id.clone(),
                nickname: actor.nickname.clone(),
                sig: actor.sig.clone(),
                kind: cat_protocol::JobKind::SupplyFood,
            })
            .await
            .expect("accepted action leaves its projection queued");
        assert!(queued.result.ok);
        let advanced = harness
            .advance_by(&mut client, 1_000)
            .await
            .expect("advance authoritative tick");
        assert_eq!(advanced.now, before + 1_000);
        let selected_id = advanced.colonies[0].id.clone();

        let reconnected = harness
            .restart_and_reconnect(client, &actor)
            .await
            .expect("restart and reconnect");
        assert_eq!(reconnected.snapshot().colonies[0].id, selected_id);
        assert!(reconnected.snapshot().colonies[0].capabilities.can_control);
        assert_eq!(
            reconnected.action_results.len(),
            3,
            "initial presence, accepted action, and reconnect presence remain traceable"
        );
    }

    #[tokio::test]
    async fn malformed_and_production_test_actions_are_observed_not_discarded() {
        let mut harness = WsGameHarness::start(4_242).await.expect("start harness");
        let (mut client, _) = harness
            .connect_authenticated("negative-install", "Negative Cat")
            .await
            .expect("authenticate over WebSocket");
        let malformed = client
            .send_raw("{not-json")
            .await
            .expect("malformed result");
        assert!(!malformed.result.ok);
        assert_eq!(malformed.result.message.as_deref(), Some("Invalid action."));
        let denied = client
            .send_action(&ClientAction::SetTestRngSeed { seed: Some(9) })
            .await
            .expect("test-control result");
        assert!(!denied.result.ok);
        assert_eq!(
            denied.result.message.as_deref(),
            Some("Test actions are disabled on this server.")
        );
        assert_eq!(client.action_results.len(), 3);
    }

    #[tokio::test]
    async fn real_socket_rate_limit_rejects_the_bounded_burst_and_recovers() {
        let mut harness = WsGameHarness::start(4_242).await.expect("start harness");
        let (mut client, actor) = harness
            .connect_authenticated("rate-limit-install", "Rate Limit Cat")
            .await
            .expect("authenticate over WebSocket");
        let action = ClientAction::RequestJob {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            kind: cat_protocol::JobKind::SupplyFood,
        };

        let mut observed = Vec::new();
        for _ in 0..=crate::ACTION_LIMIT_MAX {
            observed.push(
                client
                    .send_action(&action)
                    .await
                    .expect("burst action result"),
            );
        }
        assert_eq!(observed.len(), crate::ACTION_LIMIT_MAX + 1);
        assert_eq!(
            observed
                .last()
                .and_then(|result| result.result.message.as_deref()),
            Some("Too many actions — slow down.")
        );
        assert!(
            client.action_results.len() >= crate::ACTION_LIMIT_MAX + 2,
            "presence plus every accepted or rejected burst result is retained"
        );

        harness
            .advance_by(&mut client, crate::ACTION_LIMIT_WINDOW_MS + 1)
            .await
            .expect("advance beyond limiter window");
        let recovered = client
            .send_action(&action)
            .await
            .expect("post-window action result");
        assert_ne!(
            recovered.result.message.as_deref(),
            Some("Too many actions — slow down."),
            "the real socket limiter must prune its elapsed window"
        );
    }
}
