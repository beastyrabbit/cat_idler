//! Strict SQLite persistence for the canonical schema-v2 Leader AI aggregate.
//!
//! This is a fresh-preproduction schema boundary.  A row is either the exact
//! current [`LeaderAiRuntimeState`] aggregate or it is rejected; there is no
//! Shrine/Favor conversion, legacy cat reconciliation, shadow field, or
//! semantic migration path here.  Canonical action replay, Hole rate-limit,
//! and signed test-reset records are persisted in their own strict rows so a
//! server transaction can commit the domain aggregate and its boundary state
//! together.

use std::collections::BTreeSet;

use cat_protocol::lai64::{ReportText, StableId};
use cat_sim::{
    leader_ai_runtime::{LEADER_AI_RUNTIME_SCHEMA_VERSION, LeaderAiRuntimeState},
    world_tick::{ColonyRuntime, WorldState},
};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};

use crate::lai65::{
    CANONICAL_PERSISTENCE_ROW_SCHEMA_VERSION, CanonicalAtomicPersistenceBatch,
    CanonicalBoundaryError, CanonicalHoleClickRateRow, CanonicalReplayReceiptRow,
    CanonicalResetChallengeKey, CanonicalSessionRow,
};

/// The persistence version is deliberately independent of the runtime's own
/// schema version.  Both values must match exactly on every load.
pub const LEADER_AI_PERSISTENCE_SCHEMA_VERSION: u32 = 2;
/// Compatibility name for the outer persistence module while its callers are
/// being cut over.  It denotes the canonical schema, not an LAI.26 migration.
pub const LAI26_SCHEMA_VERSION: u32 = LEADER_AI_PERSISTENCE_SCHEMA_VERSION;
pub const LAI26_LEGACY_SOURCE_SCHEMA_VERSION: u32 = 0;
pub const LAI26_MARKER_WORLD_ID: i64 = 1;
pub const MAX_LAI26_QUARANTINE_DETAIL_BYTES: usize = 240;

const MARKER_STATUS_CANONICAL: &str = "canonical";
const QUARANTINE_REASON_MALFORMED_RUNTIME: &str = "malformed_canonical_runtime";
const QUARANTINE_REASON_NONCANONICAL_RUNTIME: &str = "noncanonical_runtime_json";
const QUARANTINE_REASON_FINGERPRINT_MISMATCH: &str = "runtime_fingerprint_mismatch";

pub type TransitionFingerprint = String;

/// Retains the name used by the outer persistence coordinator.  A missing
/// marker is not permission to migrate: it is a fresh-preproduction schema
/// failure if a world already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lai26MarkerState {
    MissingPrefeatureBoundary,
    Complete,
}

/// Strict reset-challenge persistence shape.  The signature is an already
/// verified public challenge value, never a bearer session signature or HMAC
/// secret.  Production code does not create these rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalResetChallengeRow {
    pub row_schema_version: u32,
    pub session_id: StableId,
    pub authenticated_player_id: StableId,
    pub selected_colony_id: StableId,
    pub stage_idempotency_id: StableId,
    pub nonce: StableId,
    pub signature: ReportText,
    pub expires_at_ms: i64,
}

impl CanonicalResetChallengeRow {
    pub fn validate(&self) -> Result<(), CanonicalBoundaryError> {
        if self.row_schema_version != CANONICAL_PERSISTENCE_ROW_SCHEMA_VERSION
            || self.expires_at_ms < 0
        {
            return Err(CanonicalBoundaryError::PersistenceCodec);
        }
        Ok(())
    }

    pub fn encode_json(&self) -> Result<String, CanonicalBoundaryError> {
        self.validate()?;
        serde_json::to_string(&CanonicalResetChallengeWire::from(self.clone()))
            .map_err(|_| CanonicalBoundaryError::PersistenceCodec)
    }

    pub fn decode_json(encoded: &str) -> Result<Self, CanonicalBoundaryError> {
        let wire = serde_json::from_str::<CanonicalResetChallengeWire>(encoded)
            .map_err(|_| CanonicalBoundaryError::PersistenceCodec)?;
        let row = Self::try_from(wire)?;
        row.validate()?;
        Ok(row)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CanonicalResetChallengeWire {
    row_schema_version: u32,
    session_id: StableId,
    authenticated_player_id: StableId,
    selected_colony_id: StableId,
    stage_idempotency_id: StableId,
    nonce: StableId,
    signature: ReportText,
    expires_at_ms: i64,
}

impl From<CanonicalResetChallengeRow> for CanonicalResetChallengeWire {
    fn from(value: CanonicalResetChallengeRow) -> Self {
        Self {
            row_schema_version: value.row_schema_version,
            session_id: value.session_id,
            authenticated_player_id: value.authenticated_player_id,
            selected_colony_id: value.selected_colony_id,
            stage_idempotency_id: value.stage_idempotency_id,
            nonce: value.nonce,
            signature: value.signature,
            expires_at_ms: value.expires_at_ms,
        }
    }
}

impl TryFrom<CanonicalResetChallengeWire> for CanonicalResetChallengeRow {
    type Error = CanonicalBoundaryError;

    fn try_from(value: CanonicalResetChallengeWire) -> Result<Self, Self::Error> {
        Ok(Self {
            row_schema_version: value.row_schema_version,
            session_id: value.session_id,
            authenticated_player_id: value.authenticated_player_id,
            selected_colony_id: value.selected_colony_id,
            stage_idempotency_id: value.stage_idempotency_id,
            nonce: value.nonce,
            signature: value.signature,
            expires_at_ms: value.expires_at_ms,
        })
    }
}

pub fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS leader_ai_schema_marker (
            worldId INTEGER PRIMARY KEY CHECK (worldId = 1),
            persistenceVersion INTEGER NOT NULL,
            runtimeSchemaVersion INTEGER NOT NULL,
            status TEXT NOT NULL CHECK (status = 'canonical'),
            transitionFingerprint TEXT NOT NULL,
            completedAtTick INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS leader_ai_colony_runtime (
            colonyId TEXT PRIMARY KEY,
            persistenceVersion INTEGER NOT NULL,
            runtimeSchemaVersion INTEGER NOT NULL,
            runtimeJson TEXT NOT NULL CHECK (length(runtimeJson) <= 1048576),
            runtimeFingerprint TEXT NOT NULL,
            restartValidated INTEGER NOT NULL CHECK (restartValidated IN (0, 1)),
            lastProcessedTick INTEGER
        );

        CREATE TABLE IF NOT EXISTS leader_ai_canonical_replay (
            authenticatedPlayerId TEXT NOT NULL,
            selectedColonyId TEXT NOT NULL,
            idempotencyId TEXT NOT NULL,
            rowJson TEXT NOT NULL CHECK (length(rowJson) <= 65536),
            PRIMARY KEY (authenticatedPlayerId, selectedColonyId, idempotencyId)
        );

        CREATE TABLE IF NOT EXISTS leader_ai_canonical_hole_rate (
            authenticatedPlayerId TEXT NOT NULL,
            targetId TEXT NOT NULL,
            rowJson TEXT NOT NULL CHECK (length(rowJson) <= 65536),
            PRIMARY KEY (authenticatedPlayerId, targetId)
        );

        CREATE TABLE IF NOT EXISTS leader_ai_canonical_session (
            sessionId TEXT PRIMARY KEY,
            rowJson TEXT NOT NULL CHECK (length(rowJson) <= 65536)
        );

        CREATE TABLE IF NOT EXISTS leader_ai_canonical_test_reset (
            sessionId TEXT NOT NULL,
            nonce TEXT NOT NULL,
            rowJson TEXT NOT NULL CHECK (length(rowJson) <= 65536),
            PRIMARY KEY (sessionId, nonce)
        );

        CREATE TABLE IF NOT EXISTS leader_ai_quarantine (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sourceTable TEXT NOT NULL,
            sourceRowId TEXT NOT NULL,
            reasonCode TEXT NOT NULL,
            redactedDetail TEXT NOT NULL CHECK (length(redactedDetail) <= 240),
            persistenceVersion INTEGER NOT NULL,
            createdAtTick INTEGER NOT NULL
        );
        "#,
    )?;
    for (table, expected) in [
        (
            "leader_ai_schema_marker",
            &[
                "worldId",
                "persistenceVersion",
                "runtimeSchemaVersion",
                "status",
                "transitionFingerprint",
                "completedAtTick",
            ][..],
        ),
        (
            "leader_ai_colony_runtime",
            &[
                "colonyId",
                "persistenceVersion",
                "runtimeSchemaVersion",
                "runtimeJson",
                "runtimeFingerprint",
                "restartValidated",
                "lastProcessedTick",
            ][..],
        ),
    ] {
        ensure_exact_columns(conn, table, expected)?;
    }
    Ok(())
}

fn ensure_exact_columns(conn: &Connection, table: &str, expected: &[&str]) -> rusqlite::Result<()> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let actual = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    if actual
        .iter()
        .map(String::as_str)
        .eq(expected.iter().copied())
    {
        Ok(())
    } else {
        Err(to_sql(format!(
            "fresh canonical schema required: table {table} has incompatible columns"
        )))
    }
}

pub fn begin_lai26_world_migration_transaction(
    conn: &Connection,
) -> rusqlite::Result<rusqlite::Transaction<'_>> {
    conn.unchecked_transaction()
}

pub fn commit_lai26_world_migration_transaction(
    transaction: rusqlite::Transaction<'_>,
) -> rusqlite::Result<()> {
    transaction.commit()
}

pub fn rollback_lai26_world_migration_transaction(
    transaction: rusqlite::Transaction<'_>,
) -> rusqlite::Result<()> {
    transaction.rollback()
}

pub fn validate_lai26_marker(conn: &Connection) -> rusqlite::Result<Lai26MarkerState> {
    let marker = conn
        .query_row(
            "SELECT persistenceVersion, runtimeSchemaVersion, status, transitionFingerprint
             FROM leader_ai_schema_marker WHERE worldId = ?1",
            [LAI26_MARKER_WORLD_ID],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;
    let Some((persistence, runtime, status, fingerprint)) = marker else {
        return Ok(Lai26MarkerState::MissingPrefeatureBoundary);
    };
    if persistence != i64::from(LEADER_AI_PERSISTENCE_SCHEMA_VERSION)
        || runtime != i64::from(LEADER_AI_RUNTIME_SCHEMA_VERSION)
        || status != MARKER_STATUS_CANONICAL
        || !is_sha256_fingerprint(&fingerprint)
    {
        return Err(to_sql("invalid canonical Leader AI schema marker"));
    }
    Ok(Lai26MarkerState::Complete)
}

pub fn validate_world_leader_ai_state(
    _world_seed: u32,
    colonies: &[ColonyRuntime],
) -> rusqlite::Result<Vec<PersistedLeaderAiRuntime>> {
    let mut ids = BTreeSet::new();
    let mut rows = Vec::with_capacity(colonies.len());
    for colony in colonies {
        if !ids.insert(colony.id.as_str()) {
            return Err(to_sql("duplicate canonical colony id"));
        }
        rows.push(canonical_runtime_row_for_colony(colony)?);
    }
    Ok(rows)
}

/// There is intentionally no legacy-to-canonical semantic migration.  A
/// database without the canonical marker must be recreated for preproduction.
pub fn migrate_lai26_legacy_world(
    _conn: &Connection,
    _world_seed: u32,
    _world: &mut WorldState,
) -> rusqlite::Result<()> {
    Err(to_sql(
        "legacy Leader AI saves are unsupported; recreate the fresh canonical preproduction schema",
    ))
}

pub fn save_world_leader_ai_state(
    conn: &Connection,
    world_seed: u32,
    colonies: &[ColonyRuntime],
) -> rusqlite::Result<()> {
    let rows = validate_world_leader_ai_state(world_seed, colonies)?;
    conn.execute("DELETE FROM leader_ai_colony_runtime", [])?;
    for row in &rows {
        conn.execute(
            "INSERT INTO leader_ai_colony_runtime (
                colonyId, persistenceVersion, runtimeSchemaVersion, runtimeJson,
                runtimeFingerprint, restartValidated, lastProcessedTick
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                row.colony_id,
                i64::from(LEADER_AI_PERSISTENCE_SCHEMA_VERSION),
                i64::from(LEADER_AI_RUNTIME_SCHEMA_VERSION),
                row.runtime_json,
                row.runtime_fingerprint,
                row.restart_validated,
                row.last_processed_tick
                    .map(i64::try_from)
                    .transpose()
                    .map_err(|_| {
                        to_sql("canonical runtime tick cannot be represented by SQLite")
                    })?,
            ],
        )?;
    }
    let completed_at_tick = rows
        .iter()
        .filter_map(|row| row.last_processed_tick)
        .max()
        .map(i64::try_from)
        .transpose()
        .map_err(|_| to_sql("canonical marker tick cannot be represented by SQLite"))?
        .unwrap_or_default();
    conn.execute(
        "INSERT INTO leader_ai_schema_marker (
            worldId, persistenceVersion, runtimeSchemaVersion, status,
            transitionFingerprint, completedAtTick
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(worldId) DO UPDATE SET
            persistenceVersion = excluded.persistenceVersion,
            runtimeSchemaVersion = excluded.runtimeSchemaVersion,
            status = excluded.status,
            transitionFingerprint = excluded.transitionFingerprint,
            completedAtTick = excluded.completedAtTick",
        params![
            LAI26_MARKER_WORLD_ID,
            i64::from(LEADER_AI_PERSISTENCE_SCHEMA_VERSION),
            i64::from(LEADER_AI_RUNTIME_SCHEMA_VERSION),
            MARKER_STATUS_CANONICAL,
            world_transition_fingerprint(world_seed, &rows),
            completed_at_tick,
        ],
    )?;
    Ok(())
}

pub fn load_lai26_colony_runtime(
    conn: &Connection,
    colony_id: &str,
    _world_seed: u32,
    _cats: &[cat_sim::entities::Cat],
) -> rusqlite::Result<(LeaderAiRuntimeState, bool)> {
    if validate_lai26_marker(conn)? != Lai26MarkerState::Complete {
        return Err(to_sql(
            "canonical Leader AI marker missing for persisted world; no migration is available",
        ));
    }
    let row = conn
        .query_row(
            "SELECT persistenceVersion, runtimeSchemaVersion, runtimeJson,
                    runtimeFingerprint, restartValidated
             FROM leader_ai_colony_runtime WHERE colonyId = ?1",
            [colony_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, bool>(4)?,
                ))
            },
        )
        .optional()?;
    let Some((persistence, runtime_schema, runtime_json, fingerprint, restart_validated)) = row
    else {
        return Err(to_sql(format!(
            "canonical Leader AI aggregate missing for colony {colony_id}"
        )));
    };
    if persistence != i64::from(LEADER_AI_PERSISTENCE_SCHEMA_VERSION)
        || runtime_schema != i64::from(LEADER_AI_RUNTIME_SCHEMA_VERSION)
    {
        return Err(to_sql(
            "unsupported canonical Leader AI persistence version",
        ));
    }
    let runtime = serde_json::from_str::<LeaderAiRuntimeState>(&runtime_json).map_err(|error| {
        let _ = quarantine_lai26_malformed_save(
            conn,
            "leader_ai_colony_runtime",
            colony_id,
            QUARANTINE_REASON_MALFORMED_RUNTIME,
            &error.to_string(),
        );
        to_sql("malformed canonical Leader AI aggregate")
    })?;
    runtime
        .validate()
        .map_err(|_| to_sql("invalid canonical Leader AI aggregate"))?;
    if runtime.colony_id != colony_id {
        return Err(to_sql("canonical runtime has wrong colony partition"));
    }
    let canonical_json = canonical_runtime_json(&runtime)?;
    if runtime_json != canonical_json {
        let _ = quarantine_lai26_malformed_save(
            conn,
            "leader_ai_colony_runtime",
            colony_id,
            QUARANTINE_REASON_NONCANONICAL_RUNTIME,
            "canonical JSON mismatch",
        );
        return Err(to_sql("noncanonical Leader AI aggregate JSON"));
    }
    if runtime_fingerprint(&canonical_json) != fingerprint {
        let _ = quarantine_lai26_malformed_save(
            conn,
            "leader_ai_colony_runtime",
            colony_id,
            QUARANTINE_REASON_FINGERPRINT_MISMATCH,
            "runtime fingerprint mismatch",
        );
        return Err(to_sql("canonical Leader AI aggregate fingerprint mismatch"));
    }
    Ok((runtime, restart_validated))
}

pub fn validate_lai26_no_dangling_runtime_rows<'a>(
    conn: &Connection,
    colony_ids: impl IntoIterator<Item = &'a str>,
) -> rusqlite::Result<()> {
    let canonical_ids = colony_ids.into_iter().collect::<BTreeSet<_>>();
    let mut statement =
        conn.prepare("SELECT colonyId FROM leader_ai_colony_runtime ORDER BY colonyId")?;
    for row in statement.query_map([], |row| row.get::<_, String>(0))? {
        let id = row?;
        if !canonical_ids.contains(id.as_str()) {
            return Err(to_sql("canonical runtime row references an absent colony"));
        }
    }
    Ok(())
}

pub fn quarantine_lai26_malformed_save(
    conn: &Connection,
    source_table: &str,
    source_row_id: &str,
    reason_code: &str,
    detail: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO leader_ai_quarantine (
            sourceTable, sourceRowId, reasonCode, redactedDetail,
            persistenceVersion, createdAtTick
         ) VALUES (?1, ?2, ?3, ?4, ?5, 0)",
        params![
            source_table,
            source_row_id,
            reason_code,
            bounded_quarantine_detail(detail),
            i64::from(LEADER_AI_PERSISTENCE_SCHEMA_VERSION),
        ],
    )?;
    Ok(())
}

/// Persist replay, Hole rate, session, and consumed-reset rows atomically with
/// the aggregate transaction supplied by the caller. The batch is fully
/// decoded/validated before its first write; no opaque or legacy JSON is
/// accepted.
pub fn save_canonical_boundary_batch(
    conn: &Connection,
    batch: &CanonicalAtomicPersistenceBatch,
) -> rusqlite::Result<()> {
    batch
        .validate()
        .map_err(|_| to_sql("invalid canonical boundary batch"))?;
    save_canonical_replay_row(conn, &batch.replay_row)?;
    for row in &batch.rate_rows {
        save_canonical_hole_rate_row(conn, row)?;
    }
    save_canonical_session_row(conn, &batch.session_row)?;
    if let Some(challenge) = &batch.consumed_reset_challenge {
        delete_canonical_test_reset_row(conn, challenge)?;
    }
    Ok(())
}

pub fn save_canonical_replay_row(
    conn: &Connection,
    row: &CanonicalReplayReceiptRow,
) -> rusqlite::Result<()> {
    let json = row
        .encode_json()
        .map_err(|_| to_sql("invalid canonical replay row"))?;
    conn.execute(
        "INSERT INTO leader_ai_canonical_replay (
            authenticatedPlayerId, selectedColonyId, idempotencyId, rowJson
         ) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(authenticatedPlayerId, selectedColonyId, idempotencyId)
         DO UPDATE SET rowJson = excluded.rowJson",
        params![
            row.authenticated_player_id.as_str(),
            row.selected_colony_id.as_str(),
            row.idempotency_id.as_str(),
            json,
        ],
    )?;
    Ok(())
}

pub fn save_canonical_hole_rate_row(
    conn: &Connection,
    row: &CanonicalHoleClickRateRow,
) -> rusqlite::Result<()> {
    let json = row
        .encode_json()
        .map_err(|_| to_sql("invalid canonical Hole rate row"))?;
    conn.execute(
        "INSERT INTO leader_ai_canonical_hole_rate (
            authenticatedPlayerId, targetId, rowJson
         ) VALUES (?1, ?2, ?3)
         ON CONFLICT(authenticatedPlayerId, targetId) DO UPDATE SET rowJson = excluded.rowJson",
        params![
            row.authenticated_player_id.as_str(),
            row.target_id.as_str(),
            json,
        ],
    )?;
    Ok(())
}

pub fn save_canonical_session_row(
    conn: &Connection,
    row: &CanonicalSessionRow,
) -> rusqlite::Result<()> {
    let json = row
        .encode_json()
        .map_err(|_| to_sql("invalid canonical session row"))?;
    conn.execute(
        "INSERT INTO leader_ai_canonical_session (sessionId, rowJson) VALUES (?1, ?2)
         ON CONFLICT(sessionId) DO UPDATE SET rowJson = excluded.rowJson",
        params![row.session_id.as_str(), json],
    )?;
    Ok(())
}

pub fn save_canonical_test_reset_row(
    conn: &Connection,
    row: &CanonicalResetChallengeRow,
) -> rusqlite::Result<()> {
    let json = row
        .encode_json()
        .map_err(|_| to_sql("invalid canonical test-reset row"))?;
    conn.execute(
        "INSERT INTO leader_ai_canonical_test_reset (sessionId, nonce, rowJson)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(sessionId, nonce) DO UPDATE SET rowJson = excluded.rowJson",
        params![row.session_id.as_str(), row.nonce.as_str(), json],
    )?;
    Ok(())
}

pub fn delete_canonical_test_reset_row(
    conn: &Connection,
    challenge: &CanonicalResetChallengeKey,
) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM leader_ai_canonical_test_reset WHERE sessionId = ?1 AND nonce = ?2",
        params![challenge.session_id.as_str(), challenge.nonce.as_str()],
    )?;
    Ok(())
}

pub fn load_canonical_replay_rows(
    conn: &Connection,
) -> rusqlite::Result<Vec<CanonicalReplayReceiptRow>> {
    let mut statement = conn.prepare(
        "SELECT rowJson FROM leader_ai_canonical_replay
         ORDER BY authenticatedPlayerId, selectedColonyId, idempotencyId",
    )?;
    statement
        .query_map([], |row| row.get::<_, String>(0))?
        .map(|row| {
            CanonicalReplayReceiptRow::decode_json(&row?)
                .map_err(|_| to_sql("invalid persisted canonical replay row"))
        })
        .collect()
}

pub fn load_canonical_hole_rate_rows(
    conn: &Connection,
) -> rusqlite::Result<Vec<CanonicalHoleClickRateRow>> {
    let mut statement = conn.prepare(
        "SELECT rowJson FROM leader_ai_canonical_hole_rate
         ORDER BY authenticatedPlayerId, targetId",
    )?;
    statement
        .query_map([], |row| row.get::<_, String>(0))?
        .map(|row| {
            CanonicalHoleClickRateRow::decode_json(&row?)
                .map_err(|_| to_sql("invalid persisted canonical Hole-rate row"))
        })
        .collect()
}

pub fn load_canonical_test_reset_rows(
    conn: &Connection,
) -> rusqlite::Result<Vec<CanonicalResetChallengeRow>> {
    let mut statement = conn
        .prepare("SELECT rowJson FROM leader_ai_canonical_test_reset ORDER BY sessionId, nonce")?;
    statement
        .query_map([], |row| row.get::<_, String>(0))?
        .map(|row| {
            CanonicalResetChallengeRow::decode_json(&row?)
                .map_err(|_| to_sql("invalid persisted canonical test-reset row"))
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedLeaderAiRuntime {
    pub colony_id: String,
    pub runtime_json: String,
    pub runtime_fingerprint: String,
    pub transition_fingerprint: TransitionFingerprint,
    pub restart_validated: bool,
    pub last_processed_tick: Option<u64>,
    pub completed_at_tick: i64,
}

fn canonical_runtime_row_for_colony(
    colony: &ColonyRuntime,
) -> rusqlite::Result<PersistedLeaderAiRuntime> {
    let runtime = &colony.leader_ai_runtime;
    runtime
        .validate()
        .map_err(|_| to_sql("invalid canonical Leader AI runtime aggregate"))?;
    if runtime.colony_id != colony.id {
        return Err(to_sql("canonical runtime has wrong colony partition"));
    }
    let runtime_json = canonical_runtime_json(runtime)?;
    let runtime_fingerprint = runtime_fingerprint(&runtime_json);
    Ok(PersistedLeaderAiRuntime {
        colony_id: colony.id.clone(),
        runtime_json,
        transition_fingerprint: runtime_fingerprint.clone(),
        runtime_fingerprint,
        restart_validated: colony.leader_ai_restart_validated,
        last_processed_tick: runtime.last_processed_tick,
        completed_at_tick: colony.last_tick,
    })
}

fn canonical_runtime_json(runtime: &LeaderAiRuntimeState) -> rusqlite::Result<String> {
    serde_json::to_string(runtime).map_err(|_| to_sql("failed to encode canonical runtime"))
}

fn runtime_fingerprint(runtime_json: &str) -> String {
    format!(
        "sha256:{}",
        hex_digest(Sha256::digest(runtime_json.as_bytes()).as_slice())
    )
}

fn world_transition_fingerprint(world_seed: u32, rows: &[PersistedLeaderAiRuntime]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(world_seed.to_be_bytes());
    for row in rows {
        hasher.update(row.colony_id.as_bytes());
        hasher.update([0]);
        hasher.update(row.runtime_fingerprint.as_bytes());
        hasher.update([0]);
    }
    format!("sha256:{}", hex_digest(hasher.finalize().as_slice()))
}

fn is_sha256_fingerprint(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[usize::from(byte >> 4)] as char);
        output.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    output
}

fn bounded_quarantine_detail(detail: &str) -> String {
    detail
        .chars()
        .flat_map(char::escape_default)
        .take(MAX_LAI26_QUARANTINE_DETAIL_BYTES)
        .collect()
}

fn to_sql(message: impl Into<String>) -> rusqlite::Error {
    rusqlite::Error::InvalidParameterName(message.into())
}
