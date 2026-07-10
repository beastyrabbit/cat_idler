//! SQLite persistence for `cat-server`, mirroring the relevant tables from
//! `db/schema.ts`.

use std::{collections::BTreeMap, path::Path};

use cat_sim::{
    biomes::MaxResources,
    entities::{Carrying, Cat, CatActivity, ColonyStatus, Position, Resources, RoleXp},
    officers::OfficerRole,
    skills::Labor,
    types::{BuildingType, CatSpecialization, JobKind, JobStatus, TaskType, TileType},
    upgrade_tree::{UpgradeTreeState, create_upgrade_tree_state},
    world_gen::TileResources,
    world_tick::{
        BuildingRuntime, ColonyRuntime, ConstructionPhase, ElectionKind, ElectionRuntime,
        EventKind, EventLog, JobMetadata, JobRequester, JobRuntime, RaiderRuntime, TilePos,
        VoteRuntime, WorldState, WorldTileRuntime, ZoneRuntime,
    },
    zones::{ZoneKind, ZoneRect},
};
use rusqlite::{Connection, OptionalExtension, Row, params, types::Type};
use serde_json::{Value, json};

pub fn open_database_from_env() -> rusqlite::Result<Connection> {
    let path = std::env::var("GAME_DB_PATH").unwrap_or_else(|_| "data/cat.db".to_owned());
    open_database(path)
}

pub fn open_database(path: impl AsRef<Path>) -> rusqlite::Result<Connection> {
    if let Some(parent) = path.as_ref().parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(to_sql_io)?;
    }
    let conn = Connection::open(path)?;
    init_schema(&conn)?;
    Ok(conn)
}

pub fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS world (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            worldSeed INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS colonies (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            leaderId TEXT,
            status TEXT NOT NULL,
            resources TEXT NOT NULL,
            gridSize INTEGER NOT NULL DEFAULT 9,
            createdAt INTEGER NOT NULL,
            lastTick INTEGER NOT NULL,
            lastAttack INTEGER NOT NULL DEFAULT 0,
            worldSeed INTEGER,
            isGlobal INTEGER,
            runNumber INTEGER,
            runStartedAt INTEGER,
            lastPlayerActivityAt INTEGER,
            automationTier REAL,
            globalUpgradePoints REAL,
            upgradeTree TEXT,
            upgradeLevels TEXT,
            ritualRequestedAt INTEGER,
            criticalSince INTEGER,
            claimedTiles TEXT,
            threatPressure REAL,
            lastRaidAt INTEGER,
            activeRaidId TEXT,
            raidClicks REAL,
            testTimeScale REAL,
            testResourceDecayMultiplier REAL,
            testResilienceHoursOverride REAL,
            testCriticalMsOverride INTEGER,
            testRngSeed INTEGER,
            officers TEXT
        );

        CREATE TABLE IF NOT EXISTS cats (
            id TEXT PRIMARY KEY,
            colonyId TEXT NOT NULL,
            name TEXT NOT NULL,
            parentIds TEXT NOT NULL,
            birthTime INTEGER NOT NULL,
            deathTime INTEGER,
            stats TEXT NOT NULL,
            needs TEXT NOT NULL,
            currentTask TEXT,
            position TEXT NOT NULL,
            destination TEXT,
            carrying TEXT,
            assignedBuildingId TEXT,
            activity TEXT NOT NULL,
            isPregnant INTEGER NOT NULL,
            pregnancyDueTime INTEGER,
            ageHours REAL NOT NULL DEFAULT 0,
            pregnancyDueAgeHours REAL,
            pregnancyMateId TEXT,
            spriteParams TEXT,
            specialization TEXT,
            roleXp TEXT,
            skills TEXT
        );

        CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            colonyId TEXT NOT NULL,
            kind TEXT NOT NULL,
            status TEXT NOT NULL,
            requestedByType TEXT NOT NULL,
            requestedByPlayerId TEXT,
            assignedCatId TEXT,
            baseDurationSec REAL NOT NULL,
            speedMultiplier REAL NOT NULL,
            yieldMultiplier REAL NOT NULL,
            clickTimeReducedSec REAL NOT NULL,
            createdAt INTEGER NOT NULL,
            startedAt INTEGER,
            endsAt INTEGER,
            completedAt INTEGER,
            metadata TEXT
        );

        CREATE TABLE IF NOT EXISTS buildings (
            id TEXT NOT NULL,
            colonyId TEXT NOT NULL,
            type TEXT NOT NULL,
            level INTEGER NOT NULL,
            position TEXT NOT NULL,
            constructionProgress REAL NOT NULL,
            productionProgress REAL NOT NULL DEFAULT 0,
            isComplete INTEGER NOT NULL DEFAULT 0,
            assignedCatId TEXT,
            -- Buildings are colony-scoped: type-derived ids (e.g. "shrine") are
            -- only unique within a colony, so the key is (colonyId, id).
            PRIMARY KEY (colonyId, id)
        );

        CREATE TABLE IF NOT EXISTS world_tiles (
            id TEXT PRIMARY KEY,
            colonyId TEXT NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            type TEXT NOT NULL,
            resources TEXT NOT NULL,
            maxResources TEXT NOT NULL,
            dangerLevel REAL NOT NULL,
            pathWear REAL NOT NULL,
            lastDepleted INTEGER NOT NULL,
            overlayFeature TEXT,
            UNIQUE(colonyId, x, y)
        );

        CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            colonyId TEXT NOT NULL,
            catId TEXT,
            timestamp INTEGER NOT NULL,
            type TEXT NOT NULL,
            message TEXT NOT NULL,
            involvedCatIds TEXT NOT NULL,
            metadata TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS zones (
            id TEXT PRIMARY KEY,
            colonyId TEXT NOT NULL,
            kind TEXT NOT NULL,
            x1 INTEGER NOT NULL,
            y1 INTEGER NOT NULL,
            x2 INTEGER NOT NULL,
            y2 INTEGER NOT NULL,
            playerId TEXT,
            createdAt INTEGER NOT NULL,
            expiresAt INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS elections (
            id TEXT PRIMARY KEY,
            colonyId TEXT NOT NULL,
            kind TEXT NOT NULL,
            runtimeKind TEXT NOT NULL,
            status TEXT NOT NULL,
            candidateCatIds TEXT NOT NULL,
            targetCatId TEXT,
            startedAt INTEGER NOT NULL,
            endsAt INTEGER NOT NULL,
            winnerCatId TEXT,
            resolvedAt INTEGER,
            runNumber INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS votes (
            id TEXT PRIMARY KEY,
            colonyId TEXT NOT NULL,
            electionId TEXT NOT NULL,
            playerId TEXT NOT NULL,
            catId TEXT NOT NULL,
            weight REAL NOT NULL,
            createdAt INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS raiders (
            id TEXT PRIMARY KEY,
            colonyId TEXT NOT NULL,
            raidId TEXT NOT NULL,
            position TEXT NOT NULL,
            target TEXT NOT NULL,
            strength REAL NOT NULL,
            defense REAL NOT NULL,
            hp REAL NOT NULL,
            status TEXT NOT NULL DEFAULT 'advancing',
            spawnedAt INTEGER NOT NULL
        );
        "#,
    )
}

pub fn save_world(conn: &Connection, world: &WorldState) -> rusqlite::Result<()> {
    conn.execute_batch(
        "DELETE FROM raiders;
         DELETE FROM votes;
         DELETE FROM elections;
         DELETE FROM zones;
         DELETE FROM events;
         DELETE FROM world_tiles;
         DELETE FROM buildings;
         DELETE FROM jobs;
         DELETE FROM cats;
         DELETE FROM colonies;
         DELETE FROM world;",
    )?;
    conn.execute(
        "INSERT INTO world (id, worldSeed) VALUES (1, ?1)",
        params![i64::from(world.world_seed)],
    )?;

    for colony in &world.colonies {
        save_colony(conn, world.world_seed, colony)?;
    }

    Ok(())
}

pub fn load_world(conn: &Connection) -> rusqlite::Result<Option<WorldState>> {
    let world_seed = conn
        .query_row("SELECT worldSeed FROM world WHERE id = 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .optional()?;

    let Some(world_seed) = world_seed else {
        return Ok(None);
    };

    let mut stmt = conn.prepare(
        "SELECT id, name, leaderId, status, resources, createdAt, lastTick,
                worldSeed, runNumber, runStartedAt, lastPlayerActivityAt,
                automationTier, globalUpgradePoints, upgradeTree, upgradeLevels,
                ritualRequestedAt, criticalSince, claimedTiles, threatPressure,
                lastRaidAt, activeRaidId, raidClicks, testTimeScale,
                testResourceDecayMultiplier, testResilienceHoursOverride,
                testCriticalMsOverride, testRngSeed, officers
         FROM colonies
         ORDER BY rowid",
    )?;
    let mut rows = stmt.query([])?;
    let mut colonies = Vec::new();
    while let Some(row) = rows.next()? {
        colonies.push(load_colony(conn, row)?);
    }

    Ok(Some(WorldState {
        world_seed: world_seed as u32,
        colonies,
    }))
}

fn save_colony(conn: &Connection, world_seed: u32, colony: &ColonyRuntime) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO colonies (
            id, name, leaderId, status, resources, createdAt, lastTick, worldSeed,
            runNumber, runStartedAt, lastPlayerActivityAt, automationTier,
            globalUpgradePoints, upgradeTree, upgradeLevels, ritualRequestedAt,
            criticalSince, claimedTiles, threatPressure, lastRaidAt, activeRaidId,
            raidClicks, testTimeScale, testResourceDecayMultiplier,
            testResilienceHoursOverride, testCriticalMsOverride, testRngSeed, officers
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28
        )",
        params![
            colony.id,
            colony.name,
            colony.leader_id,
            colony_status_str(colony.status),
            serde_json::to_string(&colony.resources).map_err(to_sql_json)?,
            colony.created_at,
            colony.last_tick,
            i64::from(world_seed),
            i64::from(colony.run_number),
            colony.run_started_at,
            colony.last_player_activity_at,
            colony.automation_tier,
            colony.global_upgrade_points,
            serde_json::to_string(&colony.upgrade_tree).map_err(to_sql_json)?,
            upgrade_levels_json(colony),
            colony.ritual_requested_at,
            colony.critical_since,
            tile_list_json(&colony.claimed_tiles),
            colony.threat_pressure,
            colony.last_raid_at,
            colony.active_raid,
            colony.raid_clicks,
            colony.test_time_scale,
            colony.test_resource_decay_multiplier,
            colony.test_resilience_hours_override,
            colony.test_critical_ms_override,
            colony.test_rng_seed.map(i64::from),
            serde_json::to_string(&colony.officers).map_err(to_sql_json)?,
        ],
    )?;

    for cat in &colony.cats {
        save_cat(conn, &colony.id, cat)?;
    }
    for job in &colony.jobs {
        save_job(conn, &colony.id, job)?;
    }
    for building in &colony.buildings {
        save_building(conn, &colony.id, building)?;
    }
    for tile in colony.world_tiles.values() {
        save_world_tile(conn, &colony.id, tile)?;
    }
    for event in &colony.events {
        save_event(conn, &colony.id, event)?;
    }
    for (index, zone) in colony.zones.iter().enumerate() {
        save_zone(conn, &colony.id, index, zone)?;
    }
    for election in &colony.elections {
        save_election(conn, &colony.id, colony.run_number, election)?;
    }
    for vote in &colony.votes {
        save_vote(conn, &colony.id, vote)?;
    }
    for raider in &colony.raiders {
        save_raider(conn, &colony.id, raider)?;
    }

    Ok(())
}

fn load_colony(conn: &Connection, row: &Row<'_>) -> rusqlite::Result<ColonyRuntime> {
    let id: String = row.get("id")?;
    let resources_json: String = row.get("resources")?;
    let upgrade_tree_json: Option<String> = row.get("upgradeTree")?;
    let upgrade_levels_json: Option<String> = row.get("upgradeLevels")?;
    let claimed_tiles_json: Option<String> = row.get("claimedTiles")?;
    let officers_json: Option<String> = row.get("officers")?;

    Ok(ColonyRuntime {
        id: id.clone(),
        name: row.get("name")?,
        leader_id: row.get("leaderId")?,
        status: parse_colony_status(&row.get::<_, String>("status")?)?,
        resources: serde_json::from_str::<Resources>(&resources_json).map_err(from_sql_json)?,
        cats: load_cats(conn, &id)?,
        jobs: load_jobs(conn, &id)?,
        buildings: load_buildings(conn, &id)?,
        events: load_events(conn, &id)?,
        world_tiles: load_world_tiles(conn, &id)?,
        zones: load_zones(conn, &id)?,
        elections: load_elections(conn, &id)?,
        votes: load_votes(conn, &id)?,
        raiders: load_raiders(conn, &id)?,
        upgrade_levels: parse_upgrade_levels(upgrade_levels_json.as_deref())?,
        upgrade_tree: parse_upgrade_tree(upgrade_tree_json.as_deref())?,
        automation_tier: row.get::<_, Option<f64>>("automationTier")?.unwrap_or(0.0),
        global_upgrade_points: row
            .get::<_, Option<f64>>("globalUpgradePoints")?
            .unwrap_or(0.0),
        ritual_requested_at: row.get("ritualRequestedAt")?,
        critical_since: row.get("criticalSince")?,
        claimed_tiles: parse_tile_list(claimed_tiles_json.as_deref())?,
        officers: officers_json
            .map(|raw| {
                serde_json::from_str::<BTreeMap<OfficerRole, String>>(&raw).map_err(from_sql_json)
            })
            .transpose()?
            .unwrap_or_default(),
        threat_pressure: row.get::<_, Option<f64>>("threatPressure")?.unwrap_or(0.0),
        last_raid_at: row.get("lastRaidAt")?,
        active_raid: row.get("activeRaidId")?,
        raid_clicks: row.get::<_, Option<f64>>("raidClicks")?.unwrap_or(0.0),
        run_number: row.get::<_, Option<u32>>("runNumber")?.unwrap_or(1),
        run_started_at: row.get::<_, Option<i64>>("runStartedAt")?.unwrap_or(0),
        created_at: row.get("createdAt")?,
        last_player_activity_at: row.get("lastPlayerActivityAt")?,
        last_tick: row.get("lastTick")?,
        test_time_scale: row.get::<_, Option<f64>>("testTimeScale")?.unwrap_or(1.0),
        test_resource_decay_multiplier: row
            .get::<_, Option<f64>>("testResourceDecayMultiplier")?
            .unwrap_or(1.0),
        test_resilience_hours_override: row.get("testResilienceHoursOverride")?,
        test_critical_ms_override: row
            .get::<_, Option<i64>>("testCriticalMsOverride")?
            .unwrap_or(5 * 60 * 1000),
        test_rng_seed: row.get::<_, Option<u32>>("testRngSeed")?,
    })
}

fn save_cat(conn: &Connection, colony_id: &str, cat: &Cat) -> rusqlite::Result<()> {
    let current_task = cat.current_task.map(TaskType::as_str);
    let specialization = cat.specialization.map(CatSpecialization::as_str);
    conn.execute(
        "INSERT INTO cats (
            id, colonyId, name, parentIds, birthTime, deathTime, stats, needs,
            currentTask, position, destination, carrying, activity, isPregnant,
            pregnancyDueTime, ageHours, pregnancyDueAgeHours, pregnancyMateId,
            spriteParams, specialization, roleXp, skills
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22
        )",
        params![
            cat.id,
            colony_id,
            cat.name,
            serde_json::to_string(&cat.parent_ids).map_err(to_sql_json)?,
            cat.birth_time,
            cat.death_time,
            serde_json::to_string(&cat.stats).map_err(to_sql_json)?,
            serde_json::to_string(&cat.needs).map_err(to_sql_json)?,
            current_task,
            serde_json::to_string(&cat.position).map_err(to_sql_json)?,
            cat.destination
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(to_sql_json)?,
            cat.carrying
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(to_sql_json)?,
            activity_str(cat.activity),
            cat.is_pregnant,
            cat.pregnancy_due_time,
            cat.age_hours,
            cat.pregnancy_due_age_hours,
            cat.pregnancy_mate_id,
            cat.sprite_params
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(to_sql_json)?,
            specialization,
            serde_json::to_string(&cat.role_xp).map_err(to_sql_json)?,
            serde_json::to_string(&cat.skills).map_err(to_sql_json)?,
        ],
    )?;
    Ok(())
}

fn load_cats(conn: &Connection, colony_id: &str) -> rusqlite::Result<Vec<Cat>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, parentIds, birthTime, deathTime, stats, needs,
                currentTask, position, destination, carrying, activity, isPregnant,
                pregnancyDueTime, ageHours, pregnancyDueAgeHours, pregnancyMateId,
                spriteParams, specialization, roleXp, skills
         FROM cats WHERE colonyId = ?1 ORDER BY rowid",
    )?;
    let rows = stmt.query_map([colony_id], |row| {
        let parent_ids_json: String = row.get("parentIds")?;
        let stats_json: String = row.get("stats")?;
        let needs_json: String = row.get("needs")?;
        let position_json: String = row.get("position")?;
        let role_xp_json: Option<String> = row.get("roleXp")?;
        let skills_json: Option<String> = row.get("skills")?;
        Ok(Cat {
            id: row.get("id")?,
            colony_id: colony_id.to_owned(),
            name: row.get("name")?,
            parent_ids: serde_json::from_str(&parent_ids_json).map_err(from_sql_json)?,
            birth_time: row.get("birthTime")?,
            death_time: row.get("deathTime")?,
            stats: serde_json::from_str(&stats_json).map_err(from_sql_json)?,
            needs: serde_json::from_str(&needs_json).map_err(from_sql_json)?,
            current_task: row
                .get::<_, Option<String>>("currentTask")?
                .map(|value| {
                    serde_json::from_value::<TaskType>(Value::String(value)).map_err(from_sql_json)
                })
                .transpose()?,
            position: serde_json::from_str::<Position>(&position_json).map_err(from_sql_json)?,
            destination: row
                .get::<_, Option<String>>("destination")?
                .map(|value| serde_json::from_str::<Position>(&value).map_err(from_sql_json))
                .transpose()?,
            carrying: row
                .get::<_, Option<String>>("carrying")?
                .map(|value| serde_json::from_str::<Carrying>(&value).map_err(from_sql_json))
                .transpose()?,
            activity: parse_activity(&row.get::<_, String>("activity")?)?,
            is_pregnant: row.get("isPregnant")?,
            pregnancy_due_time: row.get("pregnancyDueTime")?,
            age_hours: row.get("ageHours")?,
            pregnancy_due_age_hours: row.get("pregnancyDueAgeHours")?,
            pregnancy_mate_id: row.get("pregnancyMateId")?,
            sprite_params: row
                .get::<_, Option<String>>("spriteParams")?
                .map(|value| serde_json::from_str(&value).map_err(from_sql_json))
                .transpose()?,
            specialization: row
                .get::<_, Option<String>>("specialization")?
                .map(|value| {
                    serde_json::from_value::<CatSpecialization>(Value::String(value))
                        .map_err(from_sql_json)
                })
                .transpose()?,
            role_xp: role_xp_json
                .map(|raw| serde_json::from_str::<RoleXp>(&raw).map_err(from_sql_json))
                .transpose()?
                .unwrap_or_default(),
            skills: skills_json
                .map(|raw| {
                    serde_json::from_str::<BTreeMap<Labor, f64>>(&raw).map_err(from_sql_json)
                })
                .transpose()?
                .unwrap_or_default(),
        })
    })?;
    rows.collect()
}

fn save_job(conn: &Connection, colony_id: &str, job: &JobRuntime) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO jobs (
            id, colonyId, kind, status, requestedByType, requestedByPlayerId,
            assignedCatId, baseDurationSec, speedMultiplier, yieldMultiplier,
            clickTimeReducedSec, createdAt, startedAt, endsAt, completedAt, metadata
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15
        )",
        params![
            job.id,
            colony_id,
            job.kind.as_str(),
            job.status.as_str(),
            job_requester_str(job.requested_by),
            job.assigned_cat,
            job.duration_ms as f64 / 1000.0,
            job.speed,
            job.yield_amount,
            i64::from(job.click_count),
            job.created_at,
            job.started_at,
            job.ends_at,
            job.completed_at,
            job_metadata_json(&job.metadata).to_string(),
        ],
    )?;
    Ok(())
}

fn load_jobs(conn: &Connection, colony_id: &str) -> rusqlite::Result<Vec<JobRuntime>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, status, requestedByType, assignedCatId, baseDurationSec,
                speedMultiplier, yieldMultiplier, clickTimeReducedSec, createdAt,
                startedAt, endsAt, completedAt, metadata
         FROM jobs WHERE colonyId = ?1 ORDER BY rowid",
    )?;
    let rows = stmt.query_map([colony_id], |row| {
        let duration_sec: f64 = row.get("baseDurationSec")?;
        let click_count: f64 = row.get("clickTimeReducedSec")?;
        Ok(JobRuntime {
            id: row.get("id")?,
            kind: parse_wire_enum::<JobKind>(&row.get::<_, String>("kind")?)?,
            status: parse_wire_enum::<JobStatus>(&row.get::<_, String>("status")?)?,
            requested_by: parse_job_requester(&row.get::<_, String>("requestedByType")?),
            assigned_cat: row.get("assignedCatId")?,
            duration_ms: (duration_sec * 1000.0) as i64,
            speed: row.get("speedMultiplier")?,
            yield_amount: row.get("yieldMultiplier")?,
            click_count: click_count.max(0.0) as u32,
            created_at: row.get("createdAt")?,
            started_at: row.get("startedAt")?,
            ends_at: row.get("endsAt")?,
            completed_at: row.get("completedAt")?,
            metadata: parse_job_metadata(row.get::<_, Option<String>>("metadata")?)?,
        })
    })?;
    rows.collect()
}

fn save_building(
    conn: &Connection,
    colony_id: &str,
    building: &BuildingRuntime,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO buildings (
            id, colonyId, type, level, position, constructionProgress,
            productionProgress, isComplete, assignedCatId
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            building.id,
            colony_id,
            building.building_type.as_str(),
            i64::from(building.level),
            tile_pos_json(&building.position).to_string(),
            f64::from(building.construction_progress),
            building.production_progress,
            building.is_complete,
            building.assigned_cat,
        ],
    )?;
    Ok(())
}

fn load_buildings(conn: &Connection, colony_id: &str) -> rusqlite::Result<Vec<BuildingRuntime>> {
    let mut stmt = conn.prepare(
        "SELECT id, type, level, position, constructionProgress, productionProgress,
                isComplete, assignedCatId
         FROM buildings WHERE colonyId = ?1 ORDER BY rowid",
    )?;
    let rows = stmt.query_map([colony_id], |row| {
        let position_json: String = row.get("position")?;
        let progress: f64 = row.get("constructionProgress")?;
        Ok(BuildingRuntime {
            id: row.get("id")?,
            building_type: parse_wire_enum::<BuildingType>(&row.get::<_, String>("type")?)?,
            level: row.get("level")?,
            position: parse_tile_pos_str(&position_json)?,
            is_complete: row.get("isComplete")?,
            construction_progress: progress.clamp(0.0, 100.0) as u8,
            production_progress: row.get("productionProgress")?,
            assigned_cat: row.get("assignedCatId")?,
        })
    })?;
    rows.collect()
}

fn save_world_tile(
    conn: &Connection,
    colony_id: &str,
    tile: &WorldTileRuntime,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO world_tiles (
            id, colonyId, x, y, type, resources, maxResources, dangerLevel,
            pathWear, lastDepleted, overlayFeature
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            format!("{}:{}:{}", colony_id, tile.pos.x, tile.pos.y),
            colony_id,
            tile.pos.x,
            tile.pos.y,
            tile.tile_type.as_str(),
            tile_resources_json(tile.resources).to_string(),
            max_resources_json(tile.max_resources).to_string(),
            tile.danger_level,
            i64::from(tile.path_wear),
            tile.last_depleted,
            tile.overlay_feature,
        ],
    )?;
    Ok(())
}

fn load_world_tiles(
    conn: &Connection,
    colony_id: &str,
) -> rusqlite::Result<BTreeMap<TilePos, WorldTileRuntime>> {
    let mut stmt = conn.prepare(
        "SELECT x, y, type, resources, maxResources, dangerLevel, pathWear,
                lastDepleted, overlayFeature
         FROM world_tiles WHERE colonyId = ?1 ORDER BY x, y",
    )?;
    let rows = stmt.query_map([colony_id], |row| {
        let pos = TilePos {
            x: row.get("x")?,
            y: row.get("y")?,
        };
        let resources_json: String = row.get("resources")?;
        let max_resources_json: String = row.get("maxResources")?;
        let path_wear: f64 = row.get("pathWear")?;
        Ok((
            pos,
            WorldTileRuntime {
                pos,
                tile_type: parse_wire_enum::<TileType>(&row.get::<_, String>("type")?)?,
                resources: parse_tile_resources(&resources_json)?,
                max_resources: parse_max_resources(&max_resources_json)?,
                danger_level: row.get("dangerLevel")?,
                path_wear: path_wear.max(0.0) as u32,
                last_depleted: row.get("lastDepleted")?,
                overlay_feature: row.get("overlayFeature")?,
            },
        ))
    })?;

    let mut tiles = BTreeMap::new();
    for row in rows {
        let (pos, tile) = row?;
        tiles.insert(pos, tile);
    }
    Ok(tiles)
}

fn save_event(conn: &Connection, colony_id: &str, event: &EventLog) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO events (
            id, colonyId, timestamp, type, message, involvedCatIds, metadata
        ) VALUES (?1, ?2, ?3, ?4, ?5, '[]', '{}')",
        params![
            event.id,
            colony_id,
            event.at_ms,
            event_kind_str(&event.kind),
            event.message,
        ],
    )?;
    Ok(())
}

fn load_events(conn: &Connection, colony_id: &str) -> rusqlite::Result<Vec<EventLog>> {
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, type, message FROM events
         WHERE colonyId = ?1 ORDER BY rowid",
    )?;
    let rows = stmt.query_map([colony_id], |row| {
        Ok(EventLog {
            id: row.get("id")?,
            at_ms: row.get("timestamp")?,
            kind: parse_event_kind(&row.get::<_, String>("type")?),
            message: row.get("message")?,
        })
    })?;
    rows.collect()
}

fn save_zone(
    conn: &Connection,
    colony_id: &str,
    index: usize,
    zone: &ZoneRuntime,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO zones (
            id, colonyId, kind, x1, y1, x2, y2, playerId, createdAt, expiresAt
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            format!("zone-{}", index + 1),
            colony_id,
            zone_kind_str(zone.kind),
            zone.rect.x1,
            zone.rect.y1,
            zone.rect.x2,
            zone.rect.y2,
            zone.player_id.map(|id| id.to_string()),
            zone.created_at,
            zone.expires_at,
        ],
    )?;
    Ok(())
}

fn load_zones(conn: &Connection, colony_id: &str) -> rusqlite::Result<Vec<ZoneRuntime>> {
    let mut stmt = conn.prepare(
        "SELECT kind, x1, y1, x2, y2, playerId, createdAt, expiresAt
         FROM zones WHERE colonyId = ?1 ORDER BY rowid",
    )?;
    let rows = stmt.query_map([colony_id], |row| {
        let player_id: Option<String> = row.get("playerId")?;
        Ok(ZoneRuntime {
            rect: ZoneRect {
                x1: row.get("x1")?,
                y1: row.get("y1")?,
                x2: row.get("x2")?,
                y2: row.get("y2")?,
            },
            kind: parse_zone_kind(&row.get::<_, String>("kind")?),
            created_at: row.get("createdAt")?,
            expires_at: row.get("expiresAt")?,
            player_id: player_id.and_then(|id| id.parse().ok()),
        })
    })?;
    rows.collect()
}

fn save_election(
    conn: &Connection,
    colony_id: &str,
    run_number: u32,
    election: &ElectionRuntime,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO elections (
            id, colonyId, kind, runtimeKind, status, candidateCatIds, startedAt,
            endsAt, winnerCatId, resolvedAt, runNumber
        ) VALUES (?1, ?2, ?3, ?4, ?5, '[]', ?6, ?7, ?8, ?9, ?10)",
        params![
            election.id,
            colony_id,
            election_schema_kind(election.kind),
            election_kind_str(election.kind),
            if election.resolved_at.is_some() {
                "resolved"
            } else {
                "open"
            },
            election.opened_at,
            election.closes_at,
            election.winner_cat_id,
            election.resolved_at,
            i64::from(run_number),
        ],
    )?;
    Ok(())
}

fn load_elections(conn: &Connection, colony_id: &str) -> rusqlite::Result<Vec<ElectionRuntime>> {
    let mut stmt = conn.prepare(
        "SELECT id, runtimeKind, startedAt, endsAt, resolvedAt, winnerCatId
         FROM elections WHERE colonyId = ?1 ORDER BY rowid",
    )?;
    let rows = stmt.query_map([colony_id], |row| {
        Ok(ElectionRuntime {
            id: row.get("id")?,
            opened_at: row.get("startedAt")?,
            closes_at: row.get("endsAt")?,
            resolved_at: row.get("resolvedAt")?,
            winner_cat_id: row.get("winnerCatId")?,
            kind: parse_election_kind(&row.get::<_, String>("runtimeKind")?),
        })
    })?;
    rows.collect()
}

fn save_vote(conn: &Connection, colony_id: &str, vote: &VoteRuntime) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO votes (
            id, colonyId, electionId, playerId, catId, weight
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            vote.id,
            colony_id,
            vote.election_id,
            vote.voter_id,
            vote.cat_id,
            vote.weight,
        ],
    )?;
    Ok(())
}

fn load_votes(conn: &Connection, colony_id: &str) -> rusqlite::Result<Vec<VoteRuntime>> {
    let mut stmt = conn.prepare(
        "SELECT id, electionId, playerId, catId, weight
         FROM votes WHERE colonyId = ?1 ORDER BY rowid",
    )?;
    let rows = stmt.query_map([colony_id], |row| {
        Ok(VoteRuntime {
            id: row.get("id")?,
            election_id: row.get("electionId")?,
            voter_id: row.get("playerId")?,
            cat_id: row.get("catId")?,
            weight: row.get("weight")?,
        })
    })?;
    rows.collect()
}

fn save_raider(conn: &Connection, colony_id: &str, raider: &RaiderRuntime) -> rusqlite::Result<()> {
    let target = match &raider.destination {
        Some(destination) => serde_json::to_string(destination),
        None => serde_json::to_string(&raider.position),
    }
    .map_err(to_sql_json)?;

    conn.execute(
        "INSERT INTO raiders (
            id, colonyId, raidId, position, target, strength, defense, hp, spawnedAt
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0)",
        params![
            raider.id,
            colony_id,
            raider.raid_id,
            serde_json::to_string(&raider.position).map_err(to_sql_json)?,
            target,
            raider.attack,
            raider.defense,
            raider.health,
        ],
    )?;
    Ok(())
}

fn load_raiders(conn: &Connection, colony_id: &str) -> rusqlite::Result<Vec<RaiderRuntime>> {
    let mut stmt = conn.prepare(
        "SELECT id, raidId, position, target, strength, defense, hp
         FROM raiders WHERE colonyId = ?1 ORDER BY rowid",
    )?;
    let rows = stmt.query_map([colony_id], |row| {
        let position_json: String = row.get("position")?;
        let target_json: String = row.get("target")?;
        Ok(RaiderRuntime {
            id: row.get("id")?,
            raid_id: row.get("raidId")?,
            position: serde_json::from_str(&position_json).map_err(from_sql_json)?,
            destination: Some(serde_json::from_str(&target_json).map_err(from_sql_json)?),
            attack: row.get("strength")?,
            defense: row.get("defense")?,
            health: row.get("hp")?,
        })
    })?;
    rows.collect()
}

fn parse_wire_enum<T>(raw: &str) -> rusqlite::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    raw.parse::<T>()
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err)))
}

fn parse_colony_status(raw: &str) -> rusqlite::Result<ColonyStatus> {
    serde_json::from_value(Value::String(raw.to_owned())).map_err(from_sql_json)
}

fn parse_activity(raw: &str) -> rusqlite::Result<CatActivity> {
    serde_json::from_value(Value::String(raw.to_owned())).map_err(from_sql_json)
}

fn parse_upgrade_tree(raw: Option<&str>) -> rusqlite::Result<UpgradeTreeState> {
    raw.map(|value| serde_json::from_str(value).map_err(from_sql_json))
        .transpose()
        .map(|value| value.unwrap_or_else(create_upgrade_tree_state))
}

fn parse_upgrade_levels(raw: Option<&str>) -> rusqlite::Result<cat_sim::world_tick::UpgradeLevels> {
    let Some(raw) = raw else {
        return Ok(cat_sim::world_tick::UpgradeLevels::default());
    };
    let value: Value = serde_json::from_str(raw).map_err(from_sql_json)?;
    Ok(cat_sim::world_tick::UpgradeLevels {
        click_power: value_u32(&value, "clickPower"),
        supply_speed: value_u32(&value, "supplySpeed"),
        hunt_mastery: value_u32(&value, "huntMastery"),
        build_mastery: value_u32(&value, "buildMastery"),
        ritual_mastery: value_u32(&value, "ritualMastery"),
        resilience: value_u32(&value, "resilience"),
    })
}

fn upgrade_levels_json(colony: &ColonyRuntime) -> String {
    json!({
        "clickPower": colony.upgrade_levels.click_power,
        "supplySpeed": colony.upgrade_levels.supply_speed,
        "huntMastery": colony.upgrade_levels.hunt_mastery,
        "buildMastery": colony.upgrade_levels.build_mastery,
        "ritualMastery": colony.upgrade_levels.ritual_mastery,
        "resilience": colony.upgrade_levels.resilience,
    })
    .to_string()
}

fn value_u32(value: &Value, key: &str) -> u32 {
    value
        .get(key)
        .and_then(Value::as_u64)
        .map_or(0, |value| value as u32)
}

fn tile_list_json(tiles: &[TilePos]) -> String {
    Value::Array(tiles.iter().map(tile_pos_json).collect()).to_string()
}

fn parse_tile_list(raw: Option<&str>) -> rusqlite::Result<Vec<TilePos>> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    let value: Value = serde_json::from_str(raw).map_err(from_sql_json)?;
    let Some(items) = value.as_array() else {
        return Ok(Vec::new());
    };
    items.iter().map(parse_tile_pos_value).collect()
}

fn tile_pos_json(pos: &TilePos) -> Value {
    json!({ "x": pos.x, "y": pos.y })
}

fn parse_tile_pos_str(raw: &str) -> rusqlite::Result<TilePos> {
    let value: Value = serde_json::from_str(raw).map_err(from_sql_json)?;
    parse_tile_pos_value(&value)
}

fn parse_tile_pos_value(value: &Value) -> rusqlite::Result<TilePos> {
    Ok(TilePos {
        x: value_i32(value, "x")?,
        y: value_i32(value, "y")?,
    })
}

fn value_i32(value: &Value, key: &str) -> rusqlite::Result<i32> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .map(|value| value as i32)
        .ok_or_else(|| invalid_json(format!("missing integer {key}")))
}

fn tile_resources_json(resources: TileResources) -> Value {
    json!({
        "food": resources.food,
        "herbs": resources.herbs,
        "water": resources.water,
    })
}

fn parse_tile_resources(raw: &str) -> rusqlite::Result<TileResources> {
    let value: Value = serde_json::from_str(raw).map_err(from_sql_json)?;
    Ok(TileResources {
        food: value_u32(&value, "food"),
        herbs: value_u32(&value, "herbs"),
        water: value_u32(&value, "water"),
    })
}

fn max_resources_json(resources: MaxResources) -> Value {
    json!({
        "food": resources.food,
        "herbs": resources.herbs,
    })
}

fn parse_max_resources(raw: &str) -> rusqlite::Result<MaxResources> {
    let value: Value = serde_json::from_str(raw).map_err(from_sql_json)?;
    Ok(MaxResources {
        food: value_u32(&value, "food"),
        herbs: value_u32(&value, "herbs"),
    })
}

fn job_metadata_json(metadata: &JobMetadata) -> Value {
    match metadata {
        JobMetadata::None => json!({ "kind": "none" }),
        JobMetadata::Construction {
            phase,
            building_type,
            building_id,
            site,
        } => json!({
            "kind": "construction",
            "phase": construction_phase_str(*phase),
            "buildingType": building_type.as_str(),
            "buildingId": building_id,
            "site": site.as_ref().map(tile_pos_json),
        }),
        JobMetadata::Expansion { target, accepted } => json!({
            "kind": "expansion",
            "target": tile_pos_json(target),
            "accepted": accepted,
        }),
        JobMetadata::Hauling {
            site,
            total_yield,
            trips_done,
            next_trip_at,
            accepted,
        } => json!({
            "kind": "hauling",
            "site": site.as_ref().map(tile_pos_json),
            "totalYield": total_yield,
            "tripsDone": trips_done,
            "nextTripAt": next_trip_at,
            "accepted": accepted,
        }),
        JobMetadata::Site { site, accepted } => json!({
            "kind": "site",
            "site": tile_pos_json(site),
            "accepted": accepted,
        }),
    }
}

fn parse_job_metadata(raw: Option<String>) -> rusqlite::Result<JobMetadata> {
    let Some(raw) = raw else {
        return Ok(JobMetadata::None);
    };
    let value: Value = serde_json::from_str(&raw).map_err(from_sql_json)?;
    match value.get("kind").and_then(Value::as_str) {
        Some("construction") => Ok(JobMetadata::Construction {
            phase: parse_construction_phase(
                value
                    .get("phase")
                    .and_then(Value::as_str)
                    .unwrap_or("gatherMaterials"),
            ),
            building_type: parse_wire_enum(
                value
                    .get("buildingType")
                    .and_then(Value::as_str)
                    .unwrap_or("den"),
            )?,
            building_id: value
                .get("buildingId")
                .and_then(Value::as_str)
                .map(str::to_owned),
            site: value
                .get("site")
                .filter(|site| !site.is_null())
                .map(parse_tile_pos_value)
                .transpose()?,
        }),
        Some("expansion") => Ok(JobMetadata::Expansion {
            target: parse_tile_pos_value(value.get("target").unwrap_or(&Value::Null))?,
            accepted: value
                .get("accepted")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        Some("hauling") => Ok(JobMetadata::Hauling {
            site: value
                .get("site")
                .filter(|site| !site.is_null())
                .map(parse_tile_pos_value)
                .transpose()?,
            total_yield: value.get("totalYield").and_then(Value::as_f64),
            trips_done: value_u32(&value, "tripsDone"),
            next_trip_at: value.get("nextTripAt").and_then(Value::as_i64),
            accepted: value
                .get("accepted")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        Some("site") => Ok(JobMetadata::Site {
            site: parse_tile_pos_value(value.get("site").unwrap_or(&Value::Null))?,
            accepted: value
                .get("accepted")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        _ => Ok(JobMetadata::None),
    }
}

fn colony_status_str(status: ColonyStatus) -> &'static str {
    match status {
        ColonyStatus::Starting => "starting",
        ColonyStatus::Thriving => "thriving",
        ColonyStatus::Struggling => "struggling",
        ColonyStatus::Dead => "dead",
    }
}

fn activity_str(activity: CatActivity) -> &'static str {
    match activity {
        CatActivity::Idle => "idle",
        CatActivity::Traveling => "traveling",
        CatActivity::Working => "working",
        CatActivity::Returning => "returning",
    }
}

fn job_requester_str(requester: JobRequester) -> &'static str {
    match requester {
        JobRequester::Player => "player",
        JobRequester::Leader => "leader",
        JobRequester::System => "system",
    }
}

fn parse_job_requester(raw: &str) -> JobRequester {
    match raw {
        "player" => JobRequester::Player,
        "leader" => JobRequester::Leader,
        _ => JobRequester::System,
    }
}

fn construction_phase_str(phase: ConstructionPhase) -> &'static str {
    match phase {
        ConstructionPhase::GatherMaterials => "gatherMaterials",
        ConstructionPhase::ConstructHouse => "constructHouse",
    }
}

fn parse_construction_phase(raw: &str) -> ConstructionPhase {
    match raw {
        "constructHouse" => ConstructionPhase::ConstructHouse,
        _ => ConstructionPhase::GatherMaterials,
    }
}

fn event_kind_str(kind: &EventKind) -> &str {
    match kind {
        EventKind::LeaderChange => "leader_change",
        EventKind::JobQueued => "job_queued",
        EventKind::JobCompleted => "job_completed",
        EventKind::ResourceCrisis => "resource_crisis",
        EventKind::ResourceRecovered => "resource_recovered",
        EventKind::Election => "election",
        EventKind::Raid => "raid",
        EventKind::Reset => "reset",
        EventKind::Other(kind) => kind.as_str(),
    }
}

fn parse_event_kind(raw: &str) -> EventKind {
    match raw {
        "leader_change" => EventKind::LeaderChange,
        "job_queued" => EventKind::JobQueued,
        "job_completed" => EventKind::JobCompleted,
        "resource_crisis" => EventKind::ResourceCrisis,
        "resource_recovered" => EventKind::ResourceRecovered,
        "election" => EventKind::Election,
        "raid" => EventKind::Raid,
        "reset" => EventKind::Reset,
        other => EventKind::Other(other.to_owned()),
    }
}

fn zone_kind_str(kind: ZoneKind) -> &'static str {
    match kind {
        ZoneKind::Avoid => "avoid",
        ZoneKind::Gather => "gather",
    }
}

fn parse_zone_kind(raw: &str) -> ZoneKind {
    match raw {
        "gather" => ZoneKind::Gather,
        _ => ZoneKind::Avoid,
    }
}

fn election_kind_str(kind: ElectionKind) -> &'static str {
    match kind {
        ElectionKind::Scheduled => "scheduled",
        ElectionKind::Snap => "snap",
        ElectionKind::VoteKick => "vote_kick",
    }
}

fn election_schema_kind(kind: ElectionKind) -> &'static str {
    match kind {
        ElectionKind::VoteKick => "vote_kick",
        ElectionKind::Scheduled | ElectionKind::Snap => "election",
    }
}

fn parse_election_kind(raw: &str) -> ElectionKind {
    match raw {
        "snap" => ElectionKind::Snap,
        "vote_kick" => ElectionKind::VoteKick,
        _ => ElectionKind::Scheduled,
    }
}

fn invalid_json(message: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            message,
        )),
    )
}

fn to_sql_json(err: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(err))
}

fn from_sql_json(err: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err))
}

fn to_sql_io(err: std::io::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(err))
}

#[cfg(test)]
mod tests {
    use cat_protocol::{ClientAction, JobKind as ProtoJobKind};
    use cat_sim::{
        actions::apply_action,
        world_tick::{found_colony, new_world},
    };

    use super::*;

    #[test]
    fn save_world_load_world_round_trips_colony_resources_cats_and_jobs() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");

        let mut world = new_world(20_240_703);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000_000, 42));
        world.colonies[0].resources.food = 123.5;
        world.colonies[0].resources.water = 87.25;

        let action = ClientAction::RequestJob {
            session_id: "session-1".to_owned(),
            nickname: "Tester".to_owned(),
            sig: "ignored".to_owned(),
            kind: ProtoJobKind::SupplyFood,
        };
        let result = apply_action(
            &mut world,
            &action,
            &cat_sim::actions::ActionCtx {
                session_id: "session-1".to_owned(),
                player_id: "player-1".to_owned(),
                now_ms: 1_000_000,
            },
        );
        assert!(result.ok, "{result:?}");

        // P12.1/P12.2 state must survive the round trip.
        let officer_cat = world.colonies[0].cats[0].id.clone();
        world.colonies[0].cats[0].gain_skill(cat_sim::skills::Labor::Hunt, 3.0);
        world.colonies[0]
            .officers
            .insert(cat_sim::officers::OfficerRole::Captain, officer_cat);

        save_world(&conn, &world).expect("save world");
        let loaded = load_world(&conn)
            .expect("load world")
            .expect("world should exist");

        assert_eq!(loaded.world_seed, world.world_seed);
        assert_eq!(loaded.colonies.len(), 1);
        assert_eq!(loaded.colonies[0].resources, world.colonies[0].resources);
        assert_eq!(loaded.colonies[0].cats, world.colonies[0].cats);
        assert_eq!(loaded.colonies[0].jobs, world.colonies[0].jobs);
        assert_eq!(loaded.colonies[0].officers, world.colonies[0].officers);
    }

    #[test]
    fn legacy_colony_rows_without_officers_load_empty() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");

        let mut world = new_world(20_240_703);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000_000, 42));
        save_world(&conn, &world).expect("save world");

        // Simulate a pre-P12.2 row: officers column NULL.
        conn.execute("UPDATE colonies SET officers = NULL", [])
            .expect("null officers");
        let loaded = load_world(&conn)
            .expect("load world")
            .expect("world should exist");
        assert!(loaded.colonies[0].officers.is_empty());
    }
}
