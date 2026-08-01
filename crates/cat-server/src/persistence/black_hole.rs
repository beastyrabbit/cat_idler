//! Versioned SQLite boundary for physical Black Hole runtime state.
//!
//! This state deliberately lives outside the Leader AI aggregate so either
//! Leader implementation can issue commands without owning the Hole.

use std::collections::{BTreeMap, BTreeSet};

use cat_sim::{
    black_hole::{
        BLACK_HOLE_RUNTIME_SCHEMA_VERSION, BlackHoleRuntime, FeedKind, item_darkness_requirement,
        resource_darkness_requirement, resource_unit_value_micros,
    },
    types::BuildingType,
    world_tick::{BuildingRuntime, ColonyRuntime},
};
use rusqlite::{Connection, Row, params, types::Type};

const SEEDED_SHRINE_BUILDING_ID: &str = "building-shrine";
const MAX_RUNTIME_JSON_BYTES: usize = 1_048_576;

pub(super) fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS black_hole_runtime (
            colonyId TEXT NOT NULL,
            buildingId TEXT NOT NULL,
            runtimeSchemaVersion INTEGER NOT NULL,
            runtimeJson TEXT NOT NULL,
            PRIMARY KEY (colonyId, buildingId),
            CHECK (length(runtimeJson) <= 1048576)
        );

        CREATE INDEX IF NOT EXISTS black_hole_runtime_colony_id
            ON black_hole_runtime(colonyId);
        "#,
    )
}

pub(super) fn save_colony(conn: &Connection, colony: &ColonyRuntime) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM black_hole_runtime WHERE colonyId = ?1",
        [&colony.id],
    )?;

    let shrine_ids = shrine_building_ids(&colony.buildings);
    for (map_key, runtime) in &colony.black_holes {
        validate_runtime_reference(&colony.id, map_key, runtime, &shrine_ids, false)?;
        let runtime_json = serde_json::to_string(runtime).map_err(to_sql_json)?;
        if runtime_json.len() > MAX_RUNTIME_JSON_BYTES {
            return Err(invalid_state(format!(
                "Black Hole runtime for colony {:?}, building {:?} exceeds {} bytes",
                colony.id, map_key, MAX_RUNTIME_JSON_BYTES
            )));
        }
        conn.execute(
            "INSERT INTO black_hole_runtime (
                colonyId, buildingId, runtimeSchemaVersion, runtimeJson
             ) VALUES (?1, ?2, ?3, ?4)",
            params![
                colony.id,
                map_key,
                i64::from(BLACK_HOLE_RUNTIME_SCHEMA_VERSION),
                runtime_json,
            ],
        )?;
    }
    Ok(())
}

pub(super) fn load_colony(
    conn: &Connection,
    colony_id: &str,
    buildings: &[BuildingRuntime],
) -> rusqlite::Result<BTreeMap<String, BlackHoleRuntime>> {
    let shrine_ids = shrine_building_ids(buildings);
    let mut statement = conn.prepare(
        "SELECT buildingId, runtimeSchemaVersion, runtimeJson
         FROM black_hole_runtime
         WHERE colonyId = ?1
         ORDER BY buildingId",
    )?;
    let mut rows = statement.query([colony_id])?;
    let mut runtimes = BTreeMap::new();
    while let Some(row) = rows.next()? {
        let (building_id, runtime) = decode_runtime_row(colony_id, row)?;
        validate_runtime_reference(colony_id, &building_id, &runtime, &shrine_ids, true)?;
        if runtimes.insert(building_id.clone(), runtime).is_some() {
            return Err(corrupt_state(
                colony_id,
                &building_id,
                "duplicate Black Hole runtime row",
            ));
        }
    }

    for building in buildings.iter().filter(|building| {
        building.building_type == BuildingType::Shrine
            && (building.is_complete || building.id == SEEDED_SHRINE_BUILDING_ID)
    }) {
        runtimes
            .entry(building.id.clone())
            .or_insert_with(|| BlackHoleRuntime::for_building(building.id.clone()));
    }
    Ok(runtimes)
}

fn decode_runtime_row(
    colony_id: &str,
    row: &Row<'_>,
) -> rusqlite::Result<(String, BlackHoleRuntime)> {
    let building_id: String = row.get("buildingId")?;
    let stored_schema: i64 = row.get("runtimeSchemaVersion")?;
    if stored_schema != i64::from(BLACK_HOLE_RUNTIME_SCHEMA_VERSION) {
        return Err(corrupt_state(
            colony_id,
            &building_id,
            &format!("unsupported runtime schema version {stored_schema}"),
        ));
    }
    let runtime_json: String = row.get("runtimeJson")?;
    let runtime = serde_json::from_str::<BlackHoleRuntime>(&runtime_json).map_err(|error| {
        corrupt_state(
            colony_id,
            &building_id,
            &format!("invalid runtime JSON: {error}"),
        )
    })?;
    let canonical_json = serde_json::to_string(&runtime).map_err(to_sql_json)?;
    if canonical_json != runtime_json {
        return Err(corrupt_state(
            colony_id,
            &building_id,
            "runtime JSON is not canonical",
        ));
    }
    Ok((building_id, runtime))
}

fn shrine_building_ids(buildings: &[BuildingRuntime]) -> BTreeSet<&str> {
    buildings
        .iter()
        .filter(|building| building.building_type == BuildingType::Shrine)
        .map(|building| building.id.as_str())
        .collect()
}

fn validate_runtime_reference(
    colony_id: &str,
    map_key: &str,
    runtime: &BlackHoleRuntime,
    shrine_ids: &BTreeSet<&str>,
    loading: bool,
) -> rusqlite::Result<()> {
    if map_key.is_empty() {
        return Err(runtime_reference_error(
            loading,
            colony_id,
            map_key,
            "building id is empty",
        ));
    }
    if runtime.schema_version != BLACK_HOLE_RUNTIME_SCHEMA_VERSION {
        return Err(runtime_reference_error(
            loading,
            colony_id,
            map_key,
            "runtime schema does not match the supported version",
        ));
    }
    if runtime.building_id != map_key {
        return Err(runtime_reference_error(
            loading,
            colony_id,
            map_key,
            "map key and runtime building id differ",
        ));
    }
    if !shrine_ids.contains(map_key) {
        return Err(runtime_reference_error(
            loading,
            colony_id,
            map_key,
            "runtime does not reference a Shrine building",
        ));
    }
    if let Some(feed) = runtime.active_feed.as_ref() {
        if feed.id.is_empty() || feed.job_id.as_ref().is_some_and(String::is_empty) {
            return Err(runtime_reference_error(
                loading,
                colony_id,
                map_key,
                "active feed or job id is empty",
            ));
        }
        if feed.target_units == 0
            || feed.target_units > runtime.axes.max_order()
            || feed.delivered_units > feed.target_units
            || feed.credited_units > feed.delivered_units
        {
            return Err(runtime_reference_error(
                loading,
                colony_id,
                map_key,
                "active feed quantities violate credited <= delivered <= target <= depth capacity",
            ));
        }
        let resource_is_unlocked = resource_darkness_requirement(feed.resource)
            .is_some_and(|required| required <= runtime.axes.darkness);
        if !resource_is_unlocked {
            return Err(runtime_reference_error(
                loading,
                colony_id,
                map_key,
                "active feed resource is not accepted by the current Darkness axis",
            ));
        }
        let expected_value = u64::from(feed.credited_units)
            .saturating_mul(resource_unit_value_micros(feed.resource));
        if feed.credited_value_micros != expected_value {
            return Err(runtime_reference_error(
                loading,
                colony_id,
                map_key,
                "active feed credited value does not match its credited resource units",
            ));
        }
        if feed.created_at < 0 {
            return Err(runtime_reference_error(
                loading,
                colony_id,
                map_key,
                "active feed creation time is negative",
            ));
        }
    }
    let lifetime_quantity = runtime
        .intake
        .lifetime
        .by_kind
        .values()
        .copied()
        .try_fold(0_u64, u64::checked_add);
    let maximum_lifetime_quantity = runtime
        .intake
        .lifetime
        .openings
        .checked_mul(u64::try_from(runtime.axes.intake_width()).unwrap_or(u64::MAX));
    if lifetime_quantity != Some(runtime.intake.lifetime.quantity)
        || runtime.intake.next_opening_index != runtime.intake.lifetime.openings
        || runtime.intake.lifetime.openings > runtime.intake.lifetime.quantity
        || maximum_lifetime_quantity
            .is_none_or(|maximum| runtime.intake.lifetime.quantity > maximum)
        || runtime
            .intake
            .lifetime
            .by_kind
            .values()
            .any(|quantity| *quantity == 0)
        || runtime
            .intake
            .lifetime
            .by_kind
            .keys()
            .any(|kind| match kind {
                FeedKind::Resource { resource } => resource_darkness_requirement(*resource)
                    .is_none_or(|required| required > runtime.axes.darkness),
                FeedKind::Item { item } => {
                    item_darkness_requirement(item.kind)
                        .is_none_or(|required| required > runtime.axes.darkness)
                        || item.quality > runtime.axes.max_quality()
                }
            })
    {
        return Err(runtime_reference_error(
            loading,
            colony_id,
            map_key,
            "intake lifetime quantities and opening counters are inconsistent",
        ));
    }
    let expected_lifetime_value =
        runtime
            .intake
            .lifetime
            .by_kind
            .iter()
            .try_fold(0_u64, |total, (kind, quantity)| {
                total.checked_add(quantity.checked_mul(kind.unit_value_micros())?)
            });
    if expected_lifetime_value != Some(runtime.intake.lifetime.value_micros)
        || runtime.intake.lifetime.reward_micros != runtime.intake.lifetime.value_micros
    {
        return Err(runtime_reference_error(
            loading,
            colony_id,
            map_key,
            "intake lifetime value and reward totals are inconsistent",
        ));
    }
    if let Some(feed) = runtime
        .active_feed
        .as_ref()
        .filter(|feed| feed.credited_units > 0)
    {
        let credited_lifetime_units = runtime
            .intake
            .lifetime
            .by_kind
            .get(&FeedKind::Resource {
                resource: feed.resource,
            })
            .copied()
            .unwrap_or(0);
        if credited_lifetime_units < u64::from(feed.credited_units)
            || runtime.intake.lifetime.value_micros < feed.credited_value_micros
            || runtime.intake.lifetime.openings == 0
            || runtime.next_opening_at.is_none()
        {
            return Err(runtime_reference_error(
                loading,
                colony_id,
                map_key,
                "active feed credits exceed the persisted intake lifetime provenance",
            ));
        }
    }
    let has_openings = runtime.intake.lifetime.openings > 0;
    if runtime.next_opening_at.is_some() != has_openings
        || runtime.next_opening_at.is_some_and(|deadline| deadline < 0)
    {
        return Err(runtime_reference_error(
            loading,
            colony_id,
            map_key,
            "next-opening deadline does not match intake lifetime",
        ));
    }
    for (name, timestamp) in [
        ("urge", runtime.urged_at),
        ("review", runtime.next_review_at),
    ] {
        if timestamp.is_some_and(|value| value < 0) {
            return Err(runtime_reference_error(
                loading,
                colony_id,
                map_key,
                &format!("{name} timestamp is negative"),
            ));
        }
    }
    if let Some(project) = runtime.active_upgrade.as_ref() {
        let current = runtime.axes.level(project.axis);
        if project.job_id.is_empty()
            || project.target_level > cat_sim::black_hole::AXIS_MAX
            || project.target_level != current.saturating_add(1)
            || project.started_at < 0
        {
            return Err(runtime_reference_error(
                loading,
                colony_id,
                map_key,
                "active upgrade must target exactly one bounded next axis level",
            ));
        }
    }
    Ok(())
}

fn runtime_reference_error(
    loading: bool,
    colony_id: &str,
    building_id: &str,
    detail: &str,
) -> rusqlite::Error {
    if loading {
        corrupt_state(colony_id, building_id, detail)
    } else {
        invalid_state(format!(
            "invalid Black Hole runtime for colony {colony_id:?}, building {building_id:?}: {detail}"
        ))
    }
}

fn invalid_state(detail: String) -> rusqlite::Error {
    rusqlite::Error::InvalidParameterName(detail)
}

fn corrupt_state(colony_id: &str, building_id: &str, detail: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "corrupt Black Hole runtime for colony {colony_id:?}, building {building_id:?}: {detail}"
            ),
        )),
    )
}

fn to_sql_json(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

#[cfg(test)]
mod tests {
    use cat_sim::{
        black_hole::{
            BlackHoleAxes, BlackHoleAxis, BlackHoleFeedOrder, BlackHoleRuntime,
            BlackHoleUpgradeProject, FeedCandidate,
        },
        stockpiles::ResourceKind,
        world_tick::{found_global_colony, new_world},
    };

    use super::super::{init_schema, load_world, save_world};
    use super::*;

    fn world_with_hole() -> cat_sim::world_tick::WorldState {
        let mut world = new_world(20_260_723);
        let mut colony = found_global_colony(world.world_seed, "black-hole-colony", 1_000_000, 42);
        let building_id = colony
            .buildings
            .iter()
            .find(|building| building.building_type == BuildingType::Shrine)
            .expect("founding Shrine")
            .id
            .clone();
        let mut runtime = BlackHoleRuntime::for_building(building_id.clone());
        runtime.axes = BlackHoleAxes::new(3, 4, 5).expect("valid axes");
        let mut candidates = [FeedCandidate::resource(ResourceKind::Food, 0, 4)];
        let _ = runtime.intake.intake(runtime.axes, &mut candidates);
        runtime.active_feed = Some(BlackHoleFeedOrder {
            id: "feed-7".to_owned(),
            job_id: None,
            resource: ResourceKind::Food,
            target_units: 40,
            delivered_units: 12,
            credited_units: 4,
            credited_value_micros: 400_000,
            created_at: 1_000_100,
        });
        runtime.next_opening_at = Some(1_002_500);
        runtime.urged_at = Some(1_000_050);
        runtime.active_upgrade = Some(BlackHoleUpgradeProject {
            axis: BlackHoleAxis::Darkness,
            target_level: 6,
            job_id: "upgrade-darkness-6".to_owned(),
            started_at: 1_000_200,
        });
        runtime.next_review_at = Some(1_003_600);
        colony.black_holes.insert(building_id, runtime);
        world.colonies.push(colony);
        world
    }

    #[test]
    fn versioned_runtime_round_trips_all_fields_across_restart() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");
        let world = world_with_hole();

        save_world(&conn, &world).expect("save world");
        let restarted = load_world(&conn)
            .expect("load world")
            .expect("world exists");
        assert_eq!(
            restarted.colonies[0].black_holes,
            world.colonies[0].black_holes
        );

        save_world(&conn, &restarted).expect("save restarted world");
        let second_restart = load_world(&conn)
            .expect("reload world")
            .expect("world exists");
        assert_eq!(
            second_restart.colonies[0].black_holes,
            restarted.colonies[0].black_holes
        );
    }

    #[test]
    fn missing_runtime_row_defaults_from_the_seeded_shrine() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");
        let world = world_with_hole();
        save_world(&conn, &world).expect("save world");
        conn.execute("DELETE FROM black_hole_runtime", [])
            .expect("remove runtime rows");

        let restarted = load_world(&conn)
            .expect("load legacy world")
            .expect("world exists");
        let runtime = restarted.colonies[0]
            .black_holes
            .get(SEEDED_SHRINE_BUILDING_ID)
            .expect("default seeded Hole");
        assert_eq!(
            runtime,
            &BlackHoleRuntime::for_building(SEEDED_SHRINE_BUILDING_ID)
        );
    }

    #[test]
    fn map_key_mismatch_rejects_the_whole_save_transaction() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");
        let baseline = world_with_hole();
        save_world(&conn, &baseline).expect("save baseline");
        let persisted_baseline = load_world(&conn)
            .expect("load persisted baseline")
            .expect("baseline world exists");

        let mut invalid = persisted_baseline.clone();
        let runtime = invalid.colonies[0]
            .black_holes
            .remove(SEEDED_SHRINE_BUILDING_ID)
            .expect("seeded runtime");
        invalid.colonies[0]
            .black_holes
            .insert("wrong-building-id".to_owned(), runtime);
        assert!(save_world(&conn, &invalid).is_err());
        assert_eq!(
            load_world(&conn).expect("reload baseline"),
            Some(persisted_baseline),
            "failed Black Hole validation must roll back the enclosing world save"
        );
    }

    #[test]
    fn unsupported_row_version_fails_closed() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");
        let world = world_with_hole();
        save_world(&conn, &world).expect("save world");
        conn.execute(
            "UPDATE black_hole_runtime SET runtimeSchemaVersion = 99",
            [],
        )
        .expect("corrupt schema");

        let error = load_world(&conn).expect_err("unknown schema must fail");
        assert!(
            error.to_string().contains("unsupported runtime schema"),
            "{error}"
        );
    }

    #[test]
    fn canonical_but_semantically_invalid_upgrade_fails_closed() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");
        let world = world_with_hole();
        save_world(&conn, &world).expect("save world");
        let mut runtime = world.colonies[0].black_holes[SEEDED_SHRINE_BUILDING_ID].clone();
        runtime
            .active_upgrade
            .as_mut()
            .expect("fixture upgrade")
            .target_level = 255;
        let canonical = serde_json::to_string(&runtime).expect("canonical runtime JSON");
        conn.execute(
            "UPDATE black_hole_runtime SET runtimeJson = ?1",
            [canonical],
        )
        .expect("write semantic corruption");

        let error = load_world(&conn).expect_err("invalid upgrade must fail");
        assert!(
            error.to_string().contains("bounded next axis level"),
            "{error}"
        );
    }

    fn assert_canonical_runtime_corruption_fails_closed(
        mutate: impl FnOnce(&mut BlackHoleRuntime),
        expected_detail: &str,
    ) {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");
        let world = world_with_hole();
        save_world(&conn, &world).expect("save world");
        let mut runtime = world.colonies[0].black_holes[SEEDED_SHRINE_BUILDING_ID].clone();
        mutate(&mut runtime);
        let canonical = serde_json::to_string(&runtime).expect("canonical runtime JSON");
        conn.execute(
            "UPDATE black_hole_runtime SET runtimeJson = ?1",
            [canonical],
        )
        .expect("write semantic corruption");

        let error = load_world(&conn).expect_err("semantic corruption must fail");
        assert!(
            error.to_string().contains(expected_detail),
            "expected {expected_detail:?} in {error}"
        );
    }

    #[test]
    fn canonical_but_impossible_feed_orders_fail_closed() {
        assert_canonical_runtime_corruption_fails_closed(
            |runtime| {
                runtime.active_feed.as_mut().unwrap().resource = ResourceKind::Water;
            },
            "not accepted by the current Darkness",
        );
        assert_canonical_runtime_corruption_fails_closed(
            |runtime| {
                runtime.active_feed.as_mut().unwrap().target_units =
                    runtime.axes.max_order().saturating_add(1);
            },
            "depth capacity",
        );
        assert_canonical_runtime_corruption_fails_closed(
            |runtime| {
                runtime.active_feed.as_mut().unwrap().credited_value_micros += 1;
            },
            "credited value",
        );
    }

    #[test]
    fn canonical_but_inconsistent_intake_lifetime_fails_closed() {
        assert_canonical_runtime_corruption_fails_closed(
            |runtime| runtime.intake.lifetime.quantity += 1,
            "lifetime quantities",
        );
        assert_canonical_runtime_corruption_fails_closed(
            |runtime| runtime.next_opening_at = None,
            "lifetime provenance",
        );
        assert_canonical_runtime_corruption_fails_closed(
            |runtime| {
                runtime.intake = cat_sim::black_hole::IntakeState::default();
                runtime.next_opening_at = None;
            },
            "lifetime provenance",
        );
        assert_canonical_runtime_corruption_fails_closed(
            |runtime| {
                runtime.intake.lifetime.quantity = 5;
                runtime.intake.lifetime.value_micros = 500_000;
                runtime.intake.lifetime.reward_micros = 500_000;
                *runtime
                    .intake
                    .lifetime
                    .by_kind
                    .get_mut(&FeedKind::Resource {
                        resource: ResourceKind::Food,
                    })
                    .expect("fixture Food lifetime") = 5;
            },
            "opening counters",
        );
        assert_canonical_runtime_corruption_fails_closed(
            |runtime| {
                runtime.intake.next_opening_index = 2;
                runtime.intake.lifetime.openings = 2;
                runtime.intake.lifetime.quantity = 5;
                runtime.intake.lifetime.by_kind.insert(
                    FeedKind::Resource {
                        resource: ResourceKind::Water,
                    },
                    1,
                );
            },
            "opening counters",
        );
    }
}
