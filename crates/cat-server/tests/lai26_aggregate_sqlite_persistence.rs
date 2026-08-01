//! Focused LAI.26C aggregate persistence tests.
//!
//! Remaining parent work: do not invent LAI.25 action receipt DTO persistence here,
//! and do not perform legacy currency-to-Favor conversion until that action/version
//! contract lands.

use cat_server::{leader_ai_persistence, persistence};
use cat_sim::{
    favor::{Favor, FavorEventId, FavorEventKind},
    leader_ai_runtime::{LeaderAiRuntimeState, RuntimeIdempotencyReceipt, RuntimeMutationId},
    planner_core::PlannerId,
    world_tick::{VillageKind, VillageScale, found_colony, found_global_colony, new_world},
};
use rusqlite::Connection;

fn runtime_for(colony_id: &str, epoch: u64, receipt_suffix: &str) -> LeaderAiRuntimeState {
    let mut runtime = LeaderAiRuntimeState::new_for_colony(colony_id).unwrap();
    runtime.planner.planning_clock = epoch * 100;
    runtime.planner.planning_epoch = epoch;
    let planner_colony_id = PlannerId::derive("colony", [colony_id]);
    let receipt = RuntimeIdempotencyReceipt {
        id: RuntimeMutationId::derive("lai26-test", &planner_colony_id, receipt_suffix),
        committed_tick: epoch,
        expires_tick: epoch + 100,
        request_fingerprint: String::new(),
        response_json: String::new(),
    };
    runtime
        .idempotency_receipts
        .insert(receipt.id.clone(), receipt);
    runtime
}

fn two_colony_world() -> cat_sim::world_tick::WorldState {
    let mut world = new_world(0x1A1_260C);
    let mut global = found_global_colony(world.world_seed, "global", 1_000, 11);
    global.leader_ai_runtime = runtime_for(&global.id, 7, "global");
    global.leader_ai_restart_validated = true;

    let mut personal = found_colony(world.world_seed, "personal-owner-a", 2_000, 22);
    personal.kind = VillageKind::Personal;
    personal.scale = VillageScale::Personal;
    personal.owner_player_id = Some("player-a".to_owned());
    personal.leader_ai_runtime = runtime_for(&personal.id, 13, "personal");
    personal.leader_ai_restart_validated = true;

    world.colonies.push(global);
    world.colonies.push(personal);
    world
}

#[test]
fn aggregate_runtime_round_trips_canonically_and_isolates_colony_rows() {
    let conn = Connection::open_in_memory().unwrap();
    persistence::init_schema(&conn).unwrap();
    let world = two_colony_world();
    let expected_global = world.colonies[0].leader_ai_runtime.clone();
    let expected_personal = world.colonies[1].leader_ai_runtime.clone();

    persistence::save_world(&conn, &world).unwrap();
    let loaded = persistence::load_world(&conn).unwrap().unwrap();

    assert_eq!(loaded.colonies[0].leader_ai_runtime, expected_global);
    assert_eq!(loaded.colonies[1].leader_ai_runtime, expected_personal);
    assert!(!loaded.colonies[0].leader_ai_restart_validated);
    assert!(!loaded.colonies[1].leader_ai_restart_validated);

    let global_json: String = conn
        .query_row(
            "SELECT runtimeJson FROM leader_ai_colony_runtime WHERE colonyId = 'global'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        global_json,
        serde_json::to_string(&expected_global).unwrap()
    );
    assert!(!global_json.contains("personal-owner-a"));

    let row_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM leader_ai_colony_runtime", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(row_count, 2);
}

#[test]
fn missing_runtime_defaults_only_before_complete_lai26_marker() {
    let conn = Connection::open_in_memory().unwrap();
    persistence::init_schema(&conn).unwrap();
    let world = two_colony_world();

    persistence::save_world(&conn, &world).unwrap();
    conn.execute(
        "DELETE FROM leader_ai_colony_runtime WHERE colonyId = 'personal-owner-a'",
        [],
    )
    .unwrap();

    let err = persistence::load_world(&conn).unwrap_err();
    assert!(
        err.to_string()
            .contains("complete LAI.26 marker without runtime row")
    );

    conn.execute("DELETE FROM leader_ai_migration_marker", [])
        .unwrap();
    let loaded = persistence::load_world(&conn).unwrap().unwrap();
    assert_eq!(
        loaded.colonies[1].leader_ai_runtime.cats.len(),
        loaded.colonies[1].cats.len()
    );
}

#[test]
fn future_versions_and_partial_markers_fail_closed() {
    let conn = Connection::open_in_memory().unwrap();
    persistence::init_schema(&conn).unwrap();
    persistence::save_world(&conn, &two_colony_world()).unwrap();

    conn.execute(
        "UPDATE leader_ai_migration_marker SET persistenceVersion = ?1",
        [i64::from(leader_ai_persistence::LAI26_SCHEMA_VERSION) + 1],
    )
    .unwrap();
    assert!(persistence::load_world(&conn).is_err());

    conn.execute(
        "UPDATE leader_ai_migration_marker SET persistenceVersion = ?1, status = 'in_progress'",
        [i64::from(leader_ai_persistence::LAI26_SCHEMA_VERSION)],
    )
    .unwrap();
    assert!(persistence::load_world(&conn).is_err());
}

#[test]
fn malformed_required_aggregate_quarantines_bounded_metadata() {
    let conn = Connection::open_in_memory().unwrap();
    persistence::init_schema(&conn).unwrap();
    persistence::save_world(&conn, &two_colony_world()).unwrap();

    conn.execute(
        "UPDATE leader_ai_colony_runtime SET runtimeJson = ?1 WHERE colonyId = 'global'",
        [r#"{"schemaVersion":1,"hiddenRegenExact":123}"#],
    )
    .unwrap();

    assert!(persistence::load_world(&conn).is_err());
    let (count, max_detail): (i64, i64) = conn
        .query_row(
            "SELECT COUNT(*), MAX(length(redactedDetail)) FROM leader_ai_quarantine",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(count, 1);
    assert!(
        max_detail <= i64::from(leader_ai_persistence::MAX_LAI26_QUARANTINE_DETAIL_BYTES as u16)
    );

    let colony_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM colonies", [], |row| row.get(0))
        .unwrap();
    assert_eq!(colony_count, 2);
}

#[test]
fn invalid_aggregate_save_rolls_back_without_replacing_existing_rows() {
    let conn = Connection::open_in_memory().unwrap();
    persistence::init_schema(&conn).unwrap();
    let world = two_colony_world();
    persistence::save_world(&conn, &world).unwrap();

    let mut invalid = world.clone();
    invalid.colonies[0].leader_ai_runtime.schema_version =
        cat_sim::leader_ai_runtime::LEADER_AI_RUNTIME_SCHEMA_VERSION + 1;
    assert!(persistence::save_world(&conn, &invalid).is_err());

    let loaded = persistence::load_world(&conn).unwrap().unwrap();
    assert_eq!(
        loaded.colonies[0].leader_ai_runtime,
        world.colonies[0].leader_ai_runtime
    );
}

#[test]
fn lai26_schema_installation_is_idempotent() {
    let conn = Connection::open_in_memory().unwrap();
    persistence::init_schema(&conn).unwrap();
    persistence::init_schema(&conn).unwrap();
    persistence::save_world(&conn, &two_colony_world()).unwrap();
    persistence::init_schema(&conn).unwrap();

    let marker_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM leader_ai_migration_marker",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(marker_count, 1);
}

#[test]
fn migration_marker_records_source_target_save_identity_and_conversion_totals() {
    let conn = Connection::open_in_memory().unwrap();
    persistence::init_schema(&conn).unwrap();
    let mut world = two_colony_world();
    let global_colony_id = world.colonies[0].id.clone();
    world.colonies[0]
        .leader_ai_runtime
        .shrine_favor
        .favor
        .credit(
            FavorEventId::derive(
                "lai26_legacy_currency_conversion",
                &global_colony_id,
                "distinct",
            ),
            FavorEventKind::LegacyMigrationCredit,
            Favor::from_whole(2).unwrap(),
            0,
            7,
        )
        .unwrap();

    persistence::save_world(&conn, &world).unwrap();
    let marker: (i64, i64, String, i64, i64) = conn
        .query_row(
            "SELECT sourceSchemaVersion, targetSchemaVersion, saveIdentity,
                    conversionEventCount, conversionMicroFavorTotal
             FROM leader_ai_migration_marker WHERE worldId = 1",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        marker,
        (
            i64::from(leader_ai_persistence::LAI26_LEGACY_SOURCE_SCHEMA_VERSION),
            i64::from(leader_ai_persistence::LAI26_SCHEMA_VERSION),
            format!("world-{:08x}", world.world_seed),
            1,
            2_000_000,
        )
    );
}

#[test]
fn preview_marker_table_is_upgraded_additively() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE leader_ai_migration_marker (
            worldId INTEGER PRIMARY KEY,
            persistenceVersion INTEGER NOT NULL,
            runtimeSchemaVersion INTEGER NOT NULL,
            status TEXT NOT NULL,
            transitionFingerprint TEXT NOT NULL,
            restartValidationRequired INTEGER NOT NULL,
            completedAtTick INTEGER NOT NULL
        );",
    )
    .unwrap();

    persistence::init_schema(&conn).unwrap();
    let columns = conn
        .prepare("PRAGMA table_info(leader_ai_migration_marker)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for required in [
        "sourceSchemaVersion",
        "targetSchemaVersion",
        "saveIdentity",
        "conversionEventCount",
        "conversionMicroFavorTotal",
    ] {
        assert!(
            columns.iter().any(|column| column == required),
            "{required}"
        );
    }
}
