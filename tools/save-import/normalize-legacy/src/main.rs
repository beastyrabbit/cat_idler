//! One-time save compatibility normalizer; never used by the Unity runtime.

#[allow(dead_code)]
#[path = "../../legacy/persistence.rs"]
mod persistence;

use rusqlite::{Connection, OpenFlags, types::ValueRef};
use serde_json::{Map, Value, json};

fn boundary_edges(colony: &cat_sim::world_tick::ColonyRuntime) -> Vec<Value> {
    cat_sim::world_tick::effective_wall_segments(colony)
        .iter()
        .map(|edge| {
            let s = edge.segment;
            let (dx, dy) = match s.side {
                cat_sim::village_area::Side::N => (0, -1),
                cat_sim::village_area::Side::E => (1, 0),
                cat_sim::village_area::Side::S => (0, 1),
                cat_sim::village_area::Side::W => (-1, 0),
            };
            json!({"From":{"x":s.x,"y":s.y},"To":{"x":s.x+dx,"y":s.y+dy}})
        })
        .collect()
}
use std::{fs::OpenOptions, io::Write, path::PathBuf, time::Duration};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 2 {
        return Err("usage: forest-normalize-legacy SOURCE.sqlite NEW-DESTINATION.json".into());
    }
    let source_path = PathBuf::from(&arguments[0]).canonicalize()?;
    let destination = PathBuf::from(&arguments[1]);
    if destination.exists() {
        return Err("destination already exists".into());
    }
    let source = Connection::open_with_flags(source_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut normalized = Connection::open_in_memory()?;
    rusqlite::backup::Backup::new(&source, &mut normalized)?.run_to_completion(
        256,
        Duration::from_millis(0),
        None,
    )?;
    let integrity: String = normalized.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err("source integrity check failed".into());
    }
    // Reject future or foreign data rather than silently discarding tables/columns.
    let reference = Connection::open_in_memory()?;
    persistence::init_schema(&reference)?;
    let names = [
        "world",
        "shared_world_tiles",
        "colonies",
        "cats",
        "jobs",
        "buildings",
        "world_tiles",
        "events",
        "player_names",
        "zones",
        "elections",
        "votes",
        "raiders",
    ];
    let mut schema = normalized.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
    )?;
    let actual = schema
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for name in actual {
        if !names.contains(&name.as_str()) {
            return Err("unrecognized save table".into());
        }
        let query = format!("PRAGMA table_info({name})");
        let mut expected = reference.prepare(&query)?;
        let columns = expected
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut source_columns = normalized.prepare(&query)?;
        for column in source_columns.query_map([], |row| row.get::<_, String>(1))? {
            if !columns.contains(&column?) {
                return Err("unrecognized save column".into());
            }
        }
    }
    persistence::init_schema(&normalized)?;
    let world =
        persistence::load_world(&normalized)?.ok_or("source contains no persisted world")?;
    persistence::save_world(&normalized, &world)?;
    let mut tables = Map::new();
    for name in names {
        let mut query = normalized.prepare(&format!("SELECT * FROM {name} ORDER BY rowid"))?;
        let columns = query
            .column_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<Vec<_>>();
        let mut rows = query.query([])?;
        let mut values = Vec::new();
        while let Some(row) = rows.next()? {
            let mut object = Map::new();
            for (index, name) in columns.iter().enumerate() {
                let value = match row.get_ref(index)? {
                    ValueRef::Null => Value::Null,
                    ValueRef::Integer(value) => Value::from(value),
                    ValueRef::Real(value) => Value::from(value),
                    ValueRef::Text(value) => Value::String(std::str::from_utf8(value)?.to_owned()),
                    ValueRef::Blob(_) => {
                        return Err("unexpected blob in maintained save schema".into());
                    }
                };
                object.insert(name.clone(), value);
            }
            values.push(Value::Object(object));
        }
        tables.insert(name.to_owned(), Value::Array(values));
    }
    let mut derived = Map::new();
    for colony in &world.colonies {
        let storage = colony
            .buildings
            .iter()
            .map(|b| {
                cat_sim::storage::StorageBuilding::new(
                    b.building_type,
                    if b.is_complete {
                        100.0
                    } else {
                        f64::from(b.construction_progress)
                    },
                    Some(f64::from(b.level)),
                )
            })
            .collect::<Vec<_>>();
        let caps = cat_sim::storage::authoritative_storage_capacities_for_owned(
            &storage,
            &colony.stockpiles,
            &colony.upgrade_tree.owned_node_ids,
        );
        let piles = colony
            .stockpiles
            .iter()
            .map(|pile| {
                let limits = cat_sim::stockpiles::ResourceKind::ALL
                    .iter()
                    .map(|kind| {
                        (
                            serde_json::to_value(kind)
                                .expect("resource kind serializes")
                                .as_str()
                                .expect("resource key")
                                .to_owned(),
                            json!(cat_sim::stockpiles::capacity_for(pile, *kind, &caps)),
                        )
                    })
                    .collect::<Map<_, _>>();
                (pile.id.clone(), Value::Object(limits))
            })
            .collect::<Map<_, _>>();
        let edges = boundary_edges(colony);
        let mut expansions = Map::new();
        for job in &colony.jobs {
            if let cat_sim::world_tick::JobMetadata::Expansion {
                target,
                source_build_job_id,
                ..
            } = &job.metadata
            {
                let mut projected = colony.clone();
                let agricultural = source_build_job_id.as_ref().is_some_and(|source_id| {
                    colony.jobs.iter().any(|source| {
                        source.id == *source_id
                            && matches!(
                                source.metadata,
                                cat_sim::world_tick::JobMetadata::Construction {
                                    building_type: cat_sim::types::BuildingType::Field,
                                    ..
                                }
                            )
                    })
                });
                if !projected.claimed_tiles.contains(target) {
                    projected.claimed_tiles.push(*target);
                }
                if agricultural {
                    projected.agricultural_tiles.insert(*target);
                }
                projected.jobs.retain(|candidate| candidate.id != job.id);
                expansions.insert(
                    job.id.clone(),
                    json!({"BoundaryEdges":boundary_edges(&projected),"Agricultural":agricultural}),
                );
            }
        }
        derived.insert(
            colony.id.clone(),
            json!({"BoundaryEdges":edges,"PileResourceLimits":piles,"Expansions":expansions}),
        );
    }
    let document = json!({"Format":"idle-cat-forest-normalized-sqlite", "Version":1, "Tables":tables,"Derived":derived});
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let temporary = destination.with_extension(format!("tmp-{}", std::process::id()));
    let mut file = options.open(&temporary)?;
    let installed = (|| -> Result<(), Box<dyn std::error::Error>> {
        file.write_all(serde_json::to_string(&document)?.as_bytes())?;
        file.sync_all()?;
        // A hard link refuses to replace an existing destination, including concurrent writers.
        std::fs::hard_link(&temporary, &destination)?;
        std::fs::remove_file(&temporary)?;
        #[cfg(unix)]
        std::fs::File::open(
            destination
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| std::path::Path::new(".")),
        )?
        .sync_all()?;
        Ok(())
    })();
    if installed.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    installed?;
    println!(
        "Normalized {} villages from a read-only source. No credentials exported.",
        world.colonies.len()
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        // Errors can contain row payloads. Keep user data out of console evidence.
        eprintln!(
            "Legacy normalization failed ({}). Source data was not modified.",
            std::any::type_name_of_val(&error)
        );
        std::process::exit(1);
    }
}
