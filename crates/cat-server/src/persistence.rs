//! SQLite persistence for `cat-server`, mirroring the relevant tables from
//! `db/schema.ts`.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use cat_sim::{
    biomes::MaxResources,
    entities::{Carrying, Cat, CatActivity, ColonyStatus, Position, Resources, RoleXp},
    farming::FarmPlot,
    items::ItemStore,
    ledger::StockLedger,
    migration::MigrationState,
    officers::OfficerRole,
    skills::Labor,
    stockpiles::{FishPopulation, GatherSpot, Stockpile},
    types::{BuildingType, CatSpecialization, JobKind, JobStatus, TaskType, TileType},
    upgrade_tree::{UpgradeTreeState, create_upgrade_tree_state},
    world_gen::TileResources,
    world_tick::{
        BuildingRuntime, ColonyRuntime, ConstructionPhase, ElectionKind, ElectionRuntime,
        EventKind, EventLog, JobMetadata, JobRequester, JobRuntime, ProductionQueueEntry,
        RaiderRuntime, TilePos, VillageKind, VillageScale, VoteRuntime, WorldState,
        WorldTileRuntime, ZoneRuntime, default_production_queue, founding_revealed_tiles,
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
            foundingScale TEXT,
            runNumber INTEGER,
            runStartedAt INTEGER,
            lastPlayerActivityAt INTEGER,
            lastLoremasterUnlockAt INTEGER,
            lastTitheAt INTEGER,
            lastOfferingAt INTEGER,
            automationTier REAL,
            globalUpgradePoints REAL,
            upgradeTree TEXT,
            recipeEntitlementRulesVersion INTEGER NOT NULL DEFAULT 0,
            upgradeLevels TEXT,
            ritualRequestedAt INTEGER,
            criticalSince INTEGER,
            claimedTiles TEXT,
            agriculturalTiles TEXT,
            revealedTiles TEXT,
            provisionalTiles TEXT,
            threatPressure REAL,
            lastRaidAt INTEGER,
            activeRaidId TEXT,
            raidClicks REAL,
            testTimeScale REAL,
            testResourceDecayMultiplier REAL,
            testResilienceHoursOverride REAL,
            testCriticalMsOverride INTEGER,
            testRngSeed INTEGER,
            officers TEXT,
            stockpiles TEXT,
            farms TEXT,
            gatherSpots TEXT,
            stockLedger TEXT,
            coin REAL,
            items TEXT,
            woodCraftProgress REAL,
            stoneCraftProgress REAL,
            clothierCraftProgress REAL,
            tanneryCraftProgress REAL,
            metalForgeProgress REAL,
            anchorX INTEGER,
            anchorY INTEGER,
            migrationState TEXT,
            migrationDepartures INTEGER,
            ownerPlayerId TEXT,
            knownVillageIds TEXT,
            villageTradeOffers TEXT,
            fishHabitats TEXT
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
            skills TEXT,
            boosted INTEGER NOT NULL DEFAULT 0,
            preferredLabors TEXT
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
            automatedOfficerRole TEXT,
            productionQueue TEXT,
            productionPaused INTEGER NOT NULL DEFAULT 0,
            productionQueueInitialized INTEGER NOT NULL DEFAULT 0,
            physicalRefinerQueueInitialized INTEGER NOT NULL DEFAULT 0,
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
            revealed INTEGER NOT NULL DEFAULT 0,
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
    )?;
    migrate_add_missing_columns(conn)?;
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS colonies_one_global
            ON colonies(isGlobal) WHERE isGlobal = 1;
         CREATE UNIQUE INDEX IF NOT EXISTS colonies_one_personal_owner
            ON colonies(ownerPlayerId)
            WHERE isGlobal = 0 AND ownerPlayerId IS NOT NULL;",
    )
}

/// Add columns introduced after a database was first created. `CREATE TABLE IF NOT
/// EXISTS` never alters an existing table, so a DB made before P12 lacks these and the
/// load `SELECT` (which lists them) fails with "no such column". SQLite's `ALTER TABLE
/// ADD COLUMN` is not idempotent, so we only add the ones that are missing.
fn migrate_add_missing_columns(conn: &Connection) -> rusqlite::Result<()> {
    const ADDITIONS: &[(&str, &str, &str)] = &[
        ("colonies", "isGlobal", "INTEGER"),
        ("colonies", "foundingScale", "TEXT"),
        ("colonies", "upgradeLevels", "TEXT"),
        ("colonies", "officers", "TEXT"),
        ("colonies", "stockpiles", "TEXT"),
        ("colonies", "farms", "TEXT"),
        ("colonies", "gatherSpots", "TEXT"),
        ("colonies", "fishHabitats", "TEXT"),
        ("colonies", "stockLedger", "TEXT"),
        ("colonies", "revealedTiles", "TEXT"),
        ("colonies", "agriculturalTiles", "TEXT"),
        ("colonies", "provisionalTiles", "TEXT"),
        ("colonies", "coin", "REAL"),
        ("colonies", "items", "TEXT"),
        ("colonies", "woodCraftProgress", "REAL"),
        ("colonies", "stoneCraftProgress", "REAL"),
        ("colonies", "clothierCraftProgress", "REAL"),
        ("colonies", "tanneryCraftProgress", "REAL"),
        ("colonies", "metalForgeProgress", "REAL"),
        ("colonies", "anchorX", "INTEGER"),
        ("colonies", "anchorY", "INTEGER"),
        ("colonies", "migrationState", "TEXT"),
        ("colonies", "migrationDepartures", "INTEGER"),
        ("colonies", "ownerPlayerId", "TEXT"),
        ("colonies", "knownVillageIds", "TEXT"),
        ("colonies", "villageTradeOffers", "TEXT"),
        ("colonies", "lastLoremasterUnlockAt", "INTEGER"),
        ("colonies", "lastTitheAt", "INTEGER"),
        ("colonies", "lastOfferingAt", "INTEGER"),
        (
            "colonies",
            "recipeEntitlementRulesVersion",
            "INTEGER NOT NULL DEFAULT 0",
        ),
        ("cats", "skills", "TEXT"),
        ("cats", "boosted", "INTEGER NOT NULL DEFAULT 0"),
        ("cats", "preferredLabors", "TEXT"),
        ("world_tiles", "revealed", "INTEGER NOT NULL DEFAULT 0"),
        ("buildings", "automatedOfficerRole", "TEXT"),
        ("buildings", "productionQueue", "TEXT"),
        (
            "buildings",
            "productionPaused",
            "INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "buildings",
            "productionQueueInitialized",
            "INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "buildings",
            "physicalRefinerQueueInitialized",
            "INTEGER NOT NULL DEFAULT 0",
        ),
    ];
    for (table, column, decl) in ADDITIONS {
        if !column_exists(conn, table, column)? {
            // `table`/`column`/`decl` are compile-time constants, not user input.
            conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {decl};"))?;
            if *table == "buildings" && *column == "automatedOfficerRole" {
                // Legacy rows cannot distinguish a player-picked worker from one
                // installed by the old global automation pass. Strict-manual safety
                // wins: release every ambiguous persisted assignment once. New rows
                // have the provenance column from birth, so a genuine manual NULL is
                // never touched by later idempotent migrations.
                conn.execute(
                    "UPDATE buildings SET assignedCatId = NULL WHERE assignedCatId IS NOT NULL",
                    [],
                )?;
            }
        }
    }
    // Mill queues did not exist before the physical-chain slice. Seed every
    // uninitialized legacy Mill exactly once, then mark existing rows initialized.
    // Running this on every startup also repairs a process interrupted between the
    // column ALTER and data UPDATE. Future saves write 1, so a player-cleared [] queue
    // stays empty through every later restart.
    if column_exists(conn, "buildings", "type")? {
        conn.execute(
            "UPDATE buildings
             SET productionQueue = ?1
             WHERE type = 'mill'
               AND productionQueueInitialized = 0
               AND (productionQueue IS NULL OR productionQueue = '[]')",
            [
                serde_json::to_string(&default_production_queue(BuildingType::Mill))
                    .map_err(to_sql_json)?,
            ],
        )?;
    }
    conn.execute(
        "UPDATE buildings
         SET productionQueueInitialized = 1
         WHERE productionQueueInitialized = 0",
        [],
    )?;
    // Workshop and Smelter became editable physical stations after the original
    // queue migration had already marked every legacy building initialized. A
    // dedicated one-shot marker seeds their real repeating recipes without ever
    // repopulating a queue the player deliberately clears afterward.
    if column_exists(conn, "buildings", "type")? {
        for building_type in [BuildingType::Workshop, BuildingType::Smelter] {
            conn.execute(
                "UPDATE buildings
                 SET productionQueue = ?1
                 WHERE type = ?2
                   AND physicalRefinerQueueInitialized = 0
                   AND (productionQueue IS NULL OR productionQueue = '[]')",
                params![
                    serde_json::to_string(&default_production_queue(building_type))
                        .map_err(to_sql_json)?,
                    building_type.as_str(),
                ],
            )?;
        }
    }
    conn.execute(
        "UPDATE buildings
         SET physicalRefinerQueueInitialized = 1
         WHERE physicalRefinerQueueInitialized = 0",
        [],
    )?;
    Ok(())
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        // PRAGMA table_info columns: 0=cid, 1=name, ...
        if row.get::<_, String>(1)? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn save_world(conn: &Connection, world: &WorldState) -> rusqlite::Result<()> {
    let global_count = world
        .colonies
        .iter()
        .filter(|colony| colony.kind == VillageKind::Global)
        .count();
    if global_count != 1 {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "world must contain exactly one global village, found {global_count}"
        )));
    }
    let mut personal_owners = BTreeSet::new();
    for colony in &world.colonies {
        match colony.kind {
            VillageKind::Global if colony.owner_player_id.is_some() => {
                return Err(rusqlite::Error::InvalidParameterName(
                    "the global village cannot have a private owner".to_owned(),
                ));
            }
            VillageKind::Personal => {
                if let Some(owner) = &colony.owner_player_id
                    && !personal_owners.insert(owner.as_str())
                {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "one player cannot own multiple personal villages".to_owned(),
                    ));
                }
            }
            VillageKind::Global => {}
        }
    }
    // Replacing a world touches every persistence table. Keep the destructive
    // deletes and the complete replacement in one transaction so a failed row
    // can never strand the live database empty or partially written.
    let transaction = conn.unchecked_transaction()?;
    transaction.execute_batch(
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
    transaction.execute(
        "INSERT INTO world (id, worldSeed) VALUES (1, ?1)",
        params![i64::from(world.world_seed)],
    )?;

    for colony in &world.colonies {
        save_colony(&transaction, world.world_seed, colony)?;
    }

    transaction.commit()
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
                worldSeed, isGlobal, foundingScale, ownerPlayerId, runNumber, runStartedAt, lastPlayerActivityAt,
                lastLoremasterUnlockAt, lastTitheAt, lastOfferingAt,
                automationTier, globalUpgradePoints, upgradeTree, recipeEntitlementRulesVersion, upgradeLevels,
                ritualRequestedAt, criticalSince, claimedTiles, agriculturalTiles, revealedTiles, provisionalTiles,
                threatPressure, lastRaidAt, activeRaidId, raidClicks, testTimeScale,
                testResourceDecayMultiplier, testResilienceHoursOverride,
                testCriticalMsOverride, testRngSeed, officers, stockpiles, farms, gatherSpots,
                stockLedger, coin, items, woodCraftProgress, stoneCraftProgress,
                clothierCraftProgress, tanneryCraftProgress, metalForgeProgress, anchorX, anchorY,
                migrationState, migrationDepartures, knownVillageIds, villageTradeOffers, fishHabitats
         FROM colonies
         ORDER BY rowid",
    )?;
    let mut rows = stmt.query([])?;
    let mut colonies = Vec::new();
    while let Some(row) = rows.next()? {
        colonies.push(load_colony(conn, row)?);
    }

    if !colonies
        .iter()
        .any(|colony| colony.kind == VillageKind::Global)
    {
        let legacy_global = colonies
            .iter()
            .position(|colony| colony.id == "colony-1")
            .or_else(|| (colonies.len() == 1).then_some(0));
        if let Some(index) = legacy_global {
            colonies[index].kind = VillageKind::Global;
            colonies[index].owner_player_id = None;
        }
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
            runNumber, runStartedAt, lastPlayerActivityAt, lastLoremasterUnlockAt, lastTitheAt,
            lastOfferingAt, automationTier,
            globalUpgradePoints, upgradeTree, recipeEntitlementRulesVersion, upgradeLevels, ritualRequestedAt,
            criticalSince, claimedTiles, agriculturalTiles, revealedTiles, provisionalTiles, threatPressure, lastRaidAt,
            activeRaidId, raidClicks, testTimeScale, testResourceDecayMultiplier,
            testResilienceHoursOverride, testCriticalMsOverride, testRngSeed, officers,
            stockpiles, farms, gatherSpots, stockLedger, coin, items, woodCraftProgress,
            stoneCraftProgress, clothierCraftProgress, tanneryCraftProgress, metalForgeProgress,
            anchorX, anchorY, migrationState, migrationDepartures, isGlobal, ownerPlayerId,
            knownVillageIds, villageTradeOffers, foundingScale, fishHabitats
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31,
            ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47,
            ?48, ?49, ?50, ?51, ?52, ?53, ?54, ?55, ?56
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
            colony.last_leader_research_choice_at,
            colony.last_tithe_at,
            colony.last_offering_at,
            colony.automation_tier,
            colony.global_upgrade_points,
            serde_json::to_string(&colony.upgrade_tree).map_err(to_sql_json)?,
            i64::from(colony.recipe_entitlement_rules_version),
            upgrade_levels_json(colony),
            colony.ritual_requested_at,
            colony.critical_since,
            tile_list_json(&colony.claimed_tiles),
            tile_list_json(&colony.agricultural_tiles.iter().copied().collect::<Vec<_>>()),
            tile_list_json(&colony.revealed_tiles.iter().copied().collect::<Vec<_>>()),
            provisional_tiles_json(&colony.provisional_tiles),
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
            serde_json::to_string(&colony.stockpiles).map_err(to_sql_json)?,
            serde_json::to_string(&colony.farms).map_err(to_sql_json)?,
            serde_json::to_string(&colony.gather_spots).map_err(to_sql_json)?,
            serde_json::to_string(&colony.stock_ledger).map_err(to_sql_json)?,
            colony.coin,
            serde_json::to_string(&colony.items).map_err(to_sql_json)?,
            colony.wood_craft_progress,
            colony.stone_craft_progress,
            colony.clothier_craft_progress,
            colony.tannery_craft_progress,
            colony.metal_forge_progress,
            colony.anchor.x,
            colony.anchor.y,
            serde_json::to_string(&colony.migration_state).map_err(to_sql_json)?,
            i64::try_from(colony.migration_departures).unwrap_or(i64::MAX),
            (colony.kind == VillageKind::Global),
            colony.owner_player_id,
            serde_json::to_string(&colony.known_village_ids).map_err(to_sql_json)?,
            serde_json::to_string(&colony.village_trade_offers).map_err(to_sql_json)?,
            village_scale_str(colony.scale),
            serde_json::to_string(
                &colony
                    .fish_habitats
                    .iter()
                    .map(|(tile, population)| (tile.x, tile.y, *population))
                    .collect::<Vec<_>>(),
            )
            .map_err(to_sql_json)?,
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
    let agricultural_tiles_json: Option<String> = row.get("agriculturalTiles")?;
    let revealed_tiles_json: Option<String> = row.get("revealedTiles")?;
    let provisional_tiles_json: Option<String> = row.get("provisionalTiles")?;
    let officers_json: Option<String> = row.get("officers")?;
    let stockpiles_json: Option<String> = row.get("stockpiles")?;
    let farms_json: Option<String> = row.get("farms")?;
    let gather_spots_json: Option<String> = row.get("gatherSpots")?;
    let stock_ledger_json: Option<String> = row.get("stockLedger")?;
    let items_json: Option<String> = row.get("items")?;
    let migration_state_json: Option<String> = row.get("migrationState")?;
    let known_village_ids_json: Option<String> = row.get("knownVillageIds")?;
    let village_trade_offers_json: Option<String> = row.get("villageTradeOffers")?;
    let fish_habitats_json: Option<String> = row.get("fishHabitats")?;
    let anchor = TilePos {
        x: row.get::<_, Option<i32>>("anchorX")?.unwrap_or(6),
        y: row.get::<_, Option<i32>>("anchorY")?.unwrap_or(6),
    };
    let claimed_tiles = parse_tile_list(claimed_tiles_json.as_deref())?;
    let revealed_tiles = if revealed_tiles_json.is_some() {
        parse_tile_list(revealed_tiles_json.as_deref())?
            .into_iter()
            .collect()
    } else {
        founding_revealed_tiles(anchor, &claimed_tiles)
    };

    Ok(ColonyRuntime {
        id: id.clone(),
        name: row.get("name")?,
        kind: if row.get::<_, Option<bool>>("isGlobal")?.unwrap_or(false) {
            VillageKind::Global
        } else {
            VillageKind::Personal
        },
        scale: parse_village_scale(row.get::<_, Option<String>>("foundingScale")?.as_deref())?,
        owner_player_id: row.get("ownerPlayerId")?,
        known_village_ids: known_village_ids_json
            .map(|raw| serde_json::from_str(&raw).map_err(from_sql_json))
            .transpose()?
            .unwrap_or_default(),
        village_trade_offers: village_trade_offers_json
            .map(|raw| serde_json::from_str(&raw).map_err(from_sql_json))
            .transpose()?
            .unwrap_or_default(),
        // A complete world tick consumes shrine-delivery provenance before the
        // server persists. Reconstructing it from generic revealed tiles would
        // let expansion or legacy saves fabricate village contact.
        pending_scout_delivery_tiles: BTreeSet::new(),
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
        recipe_entitlement_rules_version: row
            .get::<_, Option<i64>>("recipeEntitlementRulesVersion")?
            .and_then(|version| u32::try_from(version).ok())
            .unwrap_or(0),
        automation_tier: row.get::<_, Option<f64>>("automationTier")?.unwrap_or(0.0),
        global_upgrade_points: row
            .get::<_, Option<f64>>("globalUpgradePoints")?
            .unwrap_or(0.0),
        ritual_requested_at: row.get("ritualRequestedAt")?,
        critical_since: row.get("criticalSince")?,
        // A colony persisted before the multi-village anchor column is colony 0 on the
        // canonical anchor (6, 6) == `village_layout::VILLAGE_ANCHOR`, so a NULL restores
        // there and keeps the single-colony game byte-identical.
        anchor,
        claimed_tiles,
        agricultural_tiles: parse_tile_list(agricultural_tiles_json.as_deref())?
            .into_iter()
            .collect(),
        revealed_tiles,
        provisional_tiles: parse_provisional_tiles(provisional_tiles_json.as_deref())?,
        officers: officers_json
            .map(|raw| {
                serde_json::from_str::<BTreeMap<OfficerRole, String>>(&raw).map_err(from_sql_json)
            })
            .transpose()?
            .unwrap_or_default(),
        stockpiles: stockpiles_json
            .map(|raw| serde_json::from_str::<Vec<Stockpile>>(&raw).map_err(from_sql_json))
            .transpose()?
            .unwrap_or_default(),
        farms: farms_json
            .map(|raw| serde_json::from_str::<Vec<FarmPlot>>(&raw).map_err(from_sql_json))
            .transpose()?
            .unwrap_or_default(),
        gather_spots: gather_spots_json
            .map(|raw| serde_json::from_str::<Vec<GatherSpot>>(&raw).map_err(from_sql_json))
            .transpose()?
            .unwrap_or_default(),
        fish_habitats: fish_habitats_json
            .map(|raw| {
                serde_json::from_str::<Vec<(i32, i32, FishPopulation)>>(&raw)
                    .map(|entries| {
                        entries
                            .into_iter()
                            .map(|(x, y, population)| (TilePos { x, y }, population))
                            .collect()
                    })
                    .map_err(from_sql_json)
            })
            .transpose()?
            .unwrap_or_default(),
        stock_ledger: stock_ledger_json
            .map(|raw| serde_json::from_str::<StockLedger>(&raw).map_err(from_sql_json))
            .transpose()?
            .unwrap_or_default(),
        // P19 slice 2 (persistence audit fix): trade-craft cycles credit real
        // player-facing goods into `items` (see `world_tick::credit_trade_craft`), so
        // it gets its own column like `coin` — losing crafted mugs/bowls/furniture/
        // trinkets/clothing on every restart was silent state loss, not a deferred
        // slice.
        items: items_json
            .map(|raw| serde_json::from_str::<ItemStore>(&raw).map_err(from_sql_json))
            .transpose()?
            .unwrap_or_default(),
        // Bench trade-craft cycle timers: persisted alongside `items` for the same
        // reason `BuildingRuntime::production_progress` is persisted — up to a full
        // cycle (900s) of accumulated progress is real player-facing state, not
        // cosmetic. `unwrap_or(0.0)` covers rows saved before this column existed.
        wood_craft_progress: row
            .get::<_, Option<f64>>("woodCraftProgress")?
            .unwrap_or(0.0),
        stone_craft_progress: row
            .get::<_, Option<f64>>("stoneCraftProgress")?
            .unwrap_or(0.0),
        clothier_craft_progress: row
            .get::<_, Option<f64>>("clothierCraftProgress")?
            .unwrap_or(0.0),
        tannery_craft_progress: row
            .get::<_, Option<f64>>("tanneryCraftProgress")?
            .unwrap_or(0.0),
        // Ore/metal: the Smelter's metal-forge sub-cycle timer, persisted alongside the
        // other craft-cycle timers (see `wood_craft_progress` above) so a restart doesn't
        // drop up to a full ~900s forge cycle. `unwrap_or(0.0)` covers rows saved before
        // this column existed.
        metal_forge_progress: row
            .get::<_, Option<f64>>("metalForgeProgress")?
            .unwrap_or(0.0),
        // P19 slice 3: `coin` is real player-facing wealth, so it gets a column (see
        // `migrate_add_missing_columns`). The in-progress trader visit + its schedule
        // reference (`last_trader_departed_at`) are NOT persisted this slice, matching
        // the `items`/craft-progress precedent above — a restart drops any mid-visit
        // trader and the next visit's game-time schedule falls back to counting from
        // `run_started_at` (see `world_tick::phase_36b_trader_lifecycle`).
        coin: row.get::<_, Option<f64>>("coin")?.unwrap_or(0.0),
        trader: None,
        last_trader_departed_at: None,
        migration_state: migration_state_json
            .map(|raw| serde_json::from_str::<MigrationState>(&raw).map_err(from_sql_json))
            .transpose()?
            .unwrap_or_default(),
        migration_departures: row
            .get::<_, Option<i64>>("migrationDepartures")?
            .unwrap_or(0)
            .max(0) as u64,
        threat_pressure: row.get::<_, Option<f64>>("threatPressure")?.unwrap_or(0.0),
        last_raid_at: row.get("lastRaidAt")?,
        active_raid: row.get("activeRaidId")?,
        raid_clicks: row.get::<_, Option<f64>>("raidClicks")?.unwrap_or(0.0),
        run_number: row.get::<_, Option<u32>>("runNumber")?.unwrap_or(1),
        run_started_at: row.get::<_, Option<i64>>("runStartedAt")?.unwrap_or(0),
        created_at: row.get("createdAt")?,
        last_player_activity_at: row.get("lastPlayerActivityAt")?,
        // Keep reading the legacy column name so existing saves retain their
        // conservative rolling-day budget after authority moves to the Leader.
        last_leader_research_choice_at: row.get("lastLoremasterUnlockAt")?,
        last_tithe_at: row.get("lastTitheAt")?,
        last_offering_at: row.get("lastOfferingAt")?,
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
        // Derived exclusively from `(world_seed, chunk)` and deliberately not
        // serialized. Loading cold avoids stale terrain data while preserving exact
        // gameplay state; movement warms the required chunks on demand.
        decoration_cache: Default::default(),
    })
}

const SCOPED_ID_SEPARATOR: char = '\u{1f}';

fn scoped_storage_id(colony_id: &str, runtime_id: &str) -> String {
    format!("{colony_id}{SCOPED_ID_SEPARATOR}{runtime_id}")
}

fn runtime_id_from_storage(colony_id: &str, stored_id: String) -> String {
    let prefix = format!("{colony_id}{SCOPED_ID_SEPARATOR}");
    stored_id
        .strip_prefix(&prefix)
        .unwrap_or(&stored_id)
        .to_owned()
}

fn save_cat(conn: &Connection, colony_id: &str, cat: &Cat) -> rusqlite::Result<()> {
    let current_task = cat.current_task.map(TaskType::as_str);
    let specialization = cat.specialization.map(CatSpecialization::as_str);
    conn.execute(
        "INSERT INTO cats (
            id, colonyId, name, parentIds, birthTime, deathTime, stats, needs,
            currentTask, position, destination, carrying, activity, isPregnant,
            pregnancyDueTime, ageHours, pregnancyDueAgeHours, pregnancyMateId,
            spriteParams, specialization, roleXp, skills, boosted, preferredLabors
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
        )",
        params![
            scoped_storage_id(colony_id, &cat.id),
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
            cat.boosted,
            serde_json::to_string(&cat.preferred_labors).map_err(to_sql_json)?,
        ],
    )?;
    Ok(())
}

fn load_cats(conn: &Connection, colony_id: &str) -> rusqlite::Result<Vec<Cat>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, parentIds, birthTime, deathTime, stats, needs,
                currentTask, position, destination, carrying, activity, isPregnant,
                pregnancyDueTime, ageHours, pregnancyDueAgeHours, pregnancyMateId,
                spriteParams, specialization, roleXp, skills, boosted, preferredLabors
         FROM cats WHERE colonyId = ?1 ORDER BY rowid",
    )?;
    let rows = stmt.query_map([colony_id], |row| {
        let parent_ids_json: String = row.get("parentIds")?;
        let stats_json: String = row.get("stats")?;
        let needs_json: String = row.get("needs")?;
        let position_json: String = row.get("position")?;
        let role_xp_json: Option<String> = row.get("roleXp")?;
        let skills_json: Option<String> = row.get("skills")?;
        let preferred_labors_json: Option<String> = row.get("preferredLabors")?;
        Ok(Cat {
            id: runtime_id_from_storage(colony_id, row.get("id")?),
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
            boosted: row.get("boosted")?,
            preferred_labors: preferred_labors_json
                .map(|raw| serde_json::from_str::<BTreeSet<Labor>>(&raw).map_err(from_sql_json))
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
            scoped_storage_id(colony_id, &job.id),
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
            id: runtime_id_from_storage(colony_id, row.get("id")?),
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
    let automated_officer_role = building
        .automated_by
        .map(|role| serde_json::to_string(&role).expect("officer role serializes"));
    conn.execute(
        "INSERT INTO buildings (
            id, colonyId, type, level, position, constructionProgress,
            productionProgress, isComplete, assignedCatId, automatedOfficerRole,
            productionQueue, productionPaused, productionQueueInitialized,
            physicalRefinerQueueInitialized
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1, 1)",
        params![
            scoped_storage_id(colony_id, &building.id),
            colony_id,
            building.building_type.as_str(),
            i64::from(building.level),
            tile_pos_json(&building.position).to_string(),
            f64::from(building.construction_progress),
            building.production_progress,
            building.is_complete,
            building.assigned_cat,
            automated_officer_role,
            serde_json::to_string(&building.production_queue).map_err(to_sql_json)?,
            building.production_paused,
        ],
    )?;
    Ok(())
}

fn load_buildings(conn: &Connection, colony_id: &str) -> rusqlite::Result<Vec<BuildingRuntime>> {
    let mut stmt = conn.prepare(
        "SELECT id, type, level, position, constructionProgress, productionProgress,
                isComplete, assignedCatId, automatedOfficerRole, productionQueue,
                productionPaused
         FROM buildings WHERE colonyId = ?1 ORDER BY rowid",
    )?;
    let rows = stmt.query_map([colony_id], |row| {
        let position_json: String = row.get("position")?;
        let progress: f64 = row.get("constructionProgress")?;
        let building_type = parse_wire_enum::<BuildingType>(&row.get::<_, String>("type")?)?;
        let production_queue = row
            .get::<_, Option<String>>("productionQueue")?
            .map(|raw| {
                serde_json::from_str::<Vec<ProductionQueueEntry>>(&raw).map_err(from_sql_json)
            })
            .transpose()?
            .unwrap_or_else(|| default_production_queue(building_type));
        Ok(BuildingRuntime {
            id: runtime_id_from_storage(colony_id, row.get("id")?),
            building_type,
            level: row.get("level")?,
            position: parse_tile_pos_str(&position_json)?,
            is_complete: row.get("isComplete")?,
            construction_progress: progress.clamp(0.0, 100.0) as u8,
            production_progress: row.get("productionProgress")?,
            assigned_cat: row.get("assignedCatId")?,
            automated_by: row
                .get::<_, Option<String>>("automatedOfficerRole")?
                .map(|raw| serde_json::from_str::<OfficerRole>(&raw).map_err(from_sql_json))
                .transpose()?,
            production_queue,
            production_paused: row.get("productionPaused")?,
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
            scoped_storage_id(colony_id, &event.id),
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
            id: runtime_id_from_storage(colony_id, row.get("id")?),
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
            scoped_storage_id(colony_id, &format!("zone-{}", index + 1)),
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
            scoped_storage_id(colony_id, &election.id),
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
            id: runtime_id_from_storage(colony_id, row.get("id")?),
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
            scoped_storage_id(colony_id, &vote.id),
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
            id: runtime_id_from_storage(colony_id, row.get("id")?),
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
            scoped_storage_id(colony_id, &raider.id),
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
            id: runtime_id_from_storage(colony_id, row.get("id")?),
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

fn provisional_tiles_json(tiles_by_scout: &BTreeMap<String, BTreeSet<TilePos>>) -> String {
    Value::Object(
        tiles_by_scout
            .iter()
            .map(|(scout_id, tiles)| {
                (
                    scout_id.clone(),
                    Value::Array(tiles.iter().map(tile_pos_json).collect()),
                )
            })
            .collect(),
    )
    .to_string()
}

fn parse_provisional_tiles(
    raw: Option<&str>,
) -> rusqlite::Result<BTreeMap<String, BTreeSet<TilePos>>> {
    let Some(raw) = raw else {
        return Ok(BTreeMap::new());
    };
    let value: Value = serde_json::from_str(raw).map_err(from_sql_json)?;
    let Some(entries) = value.as_object() else {
        return Ok(BTreeMap::new());
    };
    entries
        .iter()
        .map(|(scout_id, tiles)| {
            let items = tiles
                .as_array()
                .ok_or_else(|| invalid_json(format!("scout notebook {scout_id} is not a list")))?;
            let tiles = items
                .iter()
                .map(parse_tile_pos_value)
                .collect::<rusqlite::Result<BTreeSet<_>>>()?;
            Ok((scout_id.clone(), tiles))
        })
        .collect()
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
        JobMetadata::Expansion {
            target,
            accepted,
            source_build_job_id,
            wall_work_ms,
        } => json!({
            "kind": "expansion",
            "target": tile_pos_json(target),
            "accepted": accepted,
            "sourceBuildJobId": source_build_job_id,
            "wallWorkMs": wall_work_ms,
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
        JobMetadata::GatherHaul {
            stockpile_id,
            site,
            accepted,
        } => json!({
            "kind": "gatherHaul",
            "stockpileId": stockpile_id,
            "site": site.as_ref().map(tile_pos_json),
            "accepted": accepted,
        }),
        JobMetadata::StockpileHaul {
            source_stockpile_id,
            destination_stockpile_id,
            kind,
            site,
            accepted,
            transit_id,
            amount_in_transit,
        } => json!({
            "kind": "stockpileHaul",
            "sourceStockpileId": source_stockpile_id,
            "destinationStockpileId": destination_stockpile_id,
            "resourceKind": kind,
            "site": site.as_ref().map(tile_pos_json),
            "accepted": accepted,
            "transitId": transit_id,
            "amountInTransit": amount_in_transit,
        }),
        JobMetadata::OfferingCarry {
            source_stockpile_id,
            site,
            accepted,
            escrow_id,
            delivered,
        } => json!({
            "kind": "offeringCarry",
            "sourceStockpileId": source_stockpile_id,
            "site": site.as_ref().map(tile_pos_json),
            "accepted": accepted,
            "escrowId": escrow_id,
            "delivered": delivered,
        }),
        JobMetadata::OfferingRitual { escrow_id, amount } => json!({
            "kind": "offeringRitual",
            "escrowId": escrow_id,
            "amount": amount,
        }),
        JobMetadata::Scout {
            mission,
            target,
            destination,
            accepted,
            found,
        } => json!({
            "kind": "scout",
            "mission": scout_mission_str(*mission),
            "target": target.as_ref().map(tile_pos_json),
            "destination": destination.as_ref().map(tile_pos_json),
            "accepted": accepted,
            "found": found,
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
            source_build_job_id: value
                .get("sourceBuildJobId")
                .and_then(Value::as_str)
                .map(str::to_owned),
            wall_work_ms: value
                .get("wallWorkMs")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0),
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
        Some("gatherHaul") => Ok(JobMetadata::GatherHaul {
            stockpile_id: value
                .get("stockpileId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            site: value
                .get("site")
                .filter(|site| !site.is_null())
                .map(parse_tile_pos_value)
                .transpose()?,
            accepted: value
                .get("accepted")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        Some("stockpileHaul") => Ok(JobMetadata::StockpileHaul {
            source_stockpile_id: value
                .get("sourceStockpileId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            destination_stockpile_id: value
                .get("destinationStockpileId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            kind: serde_json::from_value(
                value
                    .get("resourceKind")
                    .cloned()
                    .unwrap_or_else(|| Value::String("food".to_owned())),
            )
            .map_err(from_sql_json)?,
            site: value
                .get("site")
                .filter(|site| !site.is_null())
                .map(parse_tile_pos_value)
                .transpose()?,
            accepted: value
                .get("accepted")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            transit_id: value
                .get("transitId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            amount_in_transit: value
                .get("amountInTransit")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .max(0.0),
        }),
        Some("offeringCarry") => Ok(JobMetadata::OfferingCarry {
            source_stockpile_id: value
                .get("sourceStockpileId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            site: value
                .get("site")
                .filter(|site| !site.is_null())
                .map(parse_tile_pos_value)
                .transpose()?,
            accepted: value
                .get("accepted")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            escrow_id: value
                .get("escrowId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            delivered: value
                .get("delivered")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .max(0.0),
        }),
        Some("offeringRitual") => Ok(JobMetadata::OfferingRitual {
            escrow_id: value
                .get("escrowId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            amount: value
                .get("amount")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .max(0.0),
        }),
        Some("scout") => Ok(JobMetadata::Scout {
            mission: parse_scout_mission(
                value
                    .get("mission")
                    .and_then(Value::as_str)
                    .unwrap_or("explore"),
            ),
            target: value
                .get("target")
                .filter(|target| !target.is_null())
                .map(parse_tile_pos_value)
                .transpose()?,
            destination: value
                .get("destination")
                .filter(|destination| !destination.is_null())
                .map(parse_tile_pos_value)
                .transpose()?,
            accepted: value
                .get("accepted")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            found: value.get("found").and_then(Value::as_bool).unwrap_or(false),
        }),
        _ => Ok(JobMetadata::None),
    }
}

fn scout_mission_str(mission: cat_sim::world_tick::ScoutMission) -> &'static str {
    use cat_sim::world_tick::{ScoutMission, ScoutResource};
    match mission {
        ScoutMission::Explore => "explore",
        ScoutMission::Resource(ScoutResource::Wood) => "wood",
        ScoutMission::Resource(ScoutResource::Food) => "food",
        ScoutMission::Resource(ScoutResource::Water) => "water",
        ScoutMission::Resource(ScoutResource::Stone) => "stone",
    }
}

fn parse_scout_mission(raw: &str) -> cat_sim::world_tick::ScoutMission {
    use cat_sim::world_tick::{ScoutMission, ScoutResource};
    match raw {
        "wood" => ScoutMission::Resource(ScoutResource::Wood),
        "food" => ScoutMission::Resource(ScoutResource::Food),
        "water" => ScoutMission::Resource(ScoutResource::Water),
        "stone" => ScoutMission::Resource(ScoutResource::Stone),
        _ => ScoutMission::Explore,
    }
}

fn village_scale_str(scale: VillageScale) -> &'static str {
    match scale {
        VillageScale::Personal => "personal",
        VillageScale::Communal => "communal",
    }
}

fn parse_village_scale(raw: Option<&str>) -> rusqlite::Result<VillageScale> {
    match raw {
        None | Some("personal") => Ok(VillageScale::Personal),
        Some("communal") => Ok(VillageScale::Communal),
        Some(other) => Err(rusqlite::Error::InvalidColumnType(
            0,
            format!("unknown foundingScale {other}"),
            Type::Text,
        )),
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

// `EventKind`'s wire taxonomy (`wire_kind` / `from_wire_kind`) is the single
// source of truth for the stable string form — reused here so the SQLite
// column and the `EventSnapshot.kind` sent to clients never drift apart.
// Pre-taxonomy rows (`"resource_crisis"`, `"election"`, `"raid"`, ...) load
// back as `EventKind::Other(raw)`, which still round-trips losslessly.
fn event_kind_str(kind: &EventKind) -> String {
    kind.wire_kind()
}

fn parse_event_kind(raw: &str) -> EventKind {
    EventKind::from_wire_kind(raw)
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
        actions::{ActionCtx, apply_action, build_snapshot},
        entities::CarryingKind,
        migration::ProbationaryMigrant,
        world_tick::{
            RaidPhase, ScoutMission, ScoutResource, found_colony, found_colony_at,
            found_global_colony, founding_revealed_tiles, new_world, world_tick,
        },
    };

    use super::*;

    fn establish_persistence_campaign_core(colony: &mut ColonyRuntime) {
        for (index, (role, building_type, upgrade)) in [
            (OfficerRole::Steward, BuildingType::Workshop, "basic_tools"),
            (OfficerRole::Forester, BuildingType::Sawmill, "sawmill"),
            (OfficerRole::Farmer, BuildingType::Field, "irrigation"),
            (OfficerRole::Captain, BuildingType::Barracks, "barracks"),
            (
                OfficerRole::Loremaster,
                BuildingType::ResearchHut,
                "research_hut",
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
            colony.buildings.push(BuildingRuntime {
                id: format!("persistence-campaign-office-{index}"),
                building_type,
                position: TilePos {
                    x: colony.anchor.x + 12 + i32::try_from(index).expect("small fixture") * 3,
                    y: colony.anchor.y + 12,
                },
                is_complete: true,
                construction_progress: 100,
                ..BuildingRuntime::default()
            });
            colony.officers.insert(role, colony.cats[index].id.clone());
        }
    }

    #[test]
    fn election_schedule_survives_restart_from_persisted_term_history() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");
        let mut world = new_world(20_260_714);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000_000, 42));
        let winner_id = world.colonies[0].cats[0].id.clone();
        world.colonies[0].test_time_scale = 20.0;
        world.colonies[0].elections.push(ElectionRuntime {
            id: "term-before-restart".to_owned(),
            opened_at: 900_000,
            closes_at: 950_000,
            resolved_at: Some(951_000),
            winner_cat_id: Some(winner_id),
            kind: ElectionKind::Scheduled,
        });
        let before = build_snapshot(&world, 1_000_000, 0).colonies[0]
            .election_schedule
            .clone()
            .expect("schedule before restart");

        save_world(&conn, &world).expect("save world");
        let loaded = load_world(&conn)
            .expect("load world")
            .expect("persisted world");
        let after = build_snapshot(&loaded, 1_000_000, 0).colonies[0]
            .election_schedule
            .clone()
            .expect("schedule after restart");

        assert_eq!(after, before);
        assert_eq!(after.term_length_ms, 4_320_000);
        assert_eq!(after.next_election_at, 5_270_000);
    }

    #[test]
    fn founding_hut_benches_and_milling_placement_truth_survive_restart() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");
        let mut world = new_world(20_260_715);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000_000, 43));
        world.colonies[0]
            .upgrade_tree
            .owned_node_ids
            .push("mill_foundations".to_owned());
        save_world(&conn, &world).expect("save world");
        let mut restarted = load_world(&conn)
            .expect("load world")
            .expect("persisted world");
        assert_eq!(
            restarted.colonies[0].upgrade_tree.owned_node_ids,
            ["mill_foundations"]
        );

        let ctx = ActionCtx {
            session_id: "restart-session".to_owned(),
            player_id: "restart-player".to_owned(),
            colony_id: "colony-1".to_owned(),
            now_ms: 1_001_000,
        };
        let plan = |building_type| ClientAction::PlanBuilding {
            session_id: ctx.session_id.clone(),
            nickname: "Restart Builder".to_owned(),
            sig: "server-verified".to_owned(),
            building_type,
            site: None,
        };
        let mut hut_world = restarted.clone();
        let hut = apply_action(
            &mut hut_world,
            &plan(cat_protocol::BuildingType::ResearchHut),
            &ctx,
        );
        assert!(hut.ok, "founding access disappeared on restart: {hut:?}");

        for bench in [
            cat_protocol::BuildingType::WoodCutter,
            cat_protocol::BuildingType::StonePrep,
            cat_protocol::BuildingType::Woodworking,
        ] {
            let mut bench_world = restarted.clone();
            let placed = apply_action(&mut bench_world, &plan(bench), &ctx);
            assert!(
                placed.ok,
                "founding {bench:?} access disappeared on restart: {placed:?}"
            );
        }

        let mill = apply_action(
            &mut restarted,
            &plan(cat_protocol::BuildingType::Mill),
            &ctx,
        );
        assert!(!mill.ok);
        assert_eq!(
            mill.message.as_deref(),
            Some("Research Milling before construction.")
        );
        restarted.colonies[0]
            .upgrade_tree
            .owned_node_ids
            .push("milling".to_owned());
        let mill = apply_action(
            &mut restarted,
            &plan(cat_protocol::BuildingType::Mill),
            &ctx,
        );
        assert!(
            mill.ok,
            "persisted colony could not place researched mill: {mill:?}"
        );
    }

    #[test]
    fn communal_and_personal_founding_scales_round_trip_without_capacity_leaks() {
        let conn = open_database(":memory:").expect("database");
        let mut world = new_world(9_999);
        let global = found_global_colony(world.world_seed, "colony-1", 10_000, 1);
        let mut personal = found_colony_at(
            world.world_seed,
            "personal",
            10_000,
            2,
            TilePos { x: 102, y: 54 },
        );
        personal.kind = VillageKind::Personal;
        personal.owner_player_id = Some("owner".to_owned());
        world.colonies = vec![global, personal];

        save_world(&conn, &world).expect("save");
        let loaded = load_world(&conn).expect("load").expect("world");
        assert_eq!(loaded.colonies[0].scale, VillageScale::Communal);
        assert_eq!(loaded.colonies[1].scale, VillageScale::Personal);
        assert_eq!(loaded.colonies[0].cats.len(), 30);
        assert_eq!(loaded.colonies[1].cats.len(), 15);
        assert_eq!(loaded.colonies[0].buildings.len(), 16);
        assert_eq!(loaded.colonies[1].buildings.len(), 7);
        assert_ne!(
            loaded.colonies[0].stockpiles[0].contents.food,
            loaded.colonies[1].stockpiles[0].contents.food
        );
        assert_eq!(loaded.colonies, world.colonies);
    }

    #[test]
    fn legacy_stack_items_migrate_to_finite_units_and_condition_survives_restart() {
        use cat_sim::items::{Item, ItemKind, Material};

        let conn = open_database(":memory:").expect("database");
        let mut world = new_world(7_711);
        world
            .colonies
            .push(found_colony(world.world_seed, "legacy-items", 10_000, 1));
        save_world(&conn, &world).expect("save baseline");
        conn.execute(
            "UPDATE colonies SET items = ?1 WHERE id = ?2",
            params![r#"{"mug:wood:1":2}"#, "legacy-items"],
        )
        .expect("install legacy item map");

        let mut migrated = load_world(&conn).expect("load").expect("world");
        let item = Item::new(ItemKind::Mug, Material::Wood, 1);
        assert_eq!(migrated.colonies[0].items.get(&item), Some(&2));
        assert_eq!(migrated.colonies[0].items.instances().count(), 2);
        let first_id = migrated.colonies[0]
            .items
            .instances()
            .next()
            .unwrap()
            .id
            .clone();
        migrated.colonies[0].items.wear(ItemKind::Mug, 1);
        let damaged = migrated.colonies[0]
            .items
            .instance(&first_id)
            .unwrap()
            .durability;

        save_world(&conn, &migrated).expect("save migrated finite ledger");
        let restarted = load_world(&conn).expect("reload").expect("world");
        assert_eq!(restarted.colonies[0].items, migrated.colonies[0].items);
        assert_eq!(
            restarted.colonies[0]
                .items
                .instance(&first_id)
                .unwrap()
                .durability,
            damaged,
            "stable identity and current condition survive SQLite restart"
        );
    }

    #[test]
    fn migration_probation_cursor_and_departure_count_survive_restart_exactly() {
        let conn = open_database(":memory:").expect("database");
        let mut world = new_world(4_242);
        let mut colony = found_colony(world.world_seed, "colony-1", 10_000, 4_242);
        colony.migration_state.last_evaluated_cohort_bucket = Some(7);
        colony
            .migration_state
            .probationary_migrants
            .push(ProbationaryMigrant {
                id: "migrant-persisted".to_owned(),
                arrived_game_minute: 1_800,
                housing_deadline_game_minute: 3_960,
            });
        colony.migration_departures = 3;
        world.colonies.push(colony);

        save_world(&conn, &world).expect("save");
        let loaded = load_world(&conn).expect("load").expect("world");

        assert_eq!(
            loaded.colonies[0].migration_state,
            world.colonies[0].migration_state
        );
        assert_eq!(loaded.colonies[0].migration_departures, 3);
    }

    #[test]
    fn station_input_transit_and_carrier_resume_exactly_after_restart() {
        let conn = open_database(":memory:").expect("database");
        let mut world = new_world(8_181);
        let mut colony = found_colony(world.world_seed, "colony-1", 10_000, 8_181);
        let building_id = "restart-sawmill";
        colony.buildings.push(BuildingRuntime {
            id: building_id.to_owned(),
            building_type: BuildingType::Sawmill,
            position: TilePos { x: 18, y: 18 },
            is_complete: true,
            construction_progress: 100,
            production_progress: 317.5,
            assigned_cat: Some(colony.cats[0].id.clone()),
            production_queue: vec![ProductionQueueEntry {
                recipe_id: cat_sim::world_tick::SAWMILL_RECIPE_ID.to_owned(),
                repeat: false,
            }],
            production_paused: true,
            ..BuildingRuntime::default()
        });
        let transit_id = cat_sim::stockpiles::station_transit_id(building_id);
        colony.stockpiles.push(Stockpile {
            id: transit_id.clone(),
            rect: ZoneRect {
                x1: 11,
                y1: 2,
                x2: 13,
                y2: 4,
            },
            accepts: [cat_sim::stockpiles::ResourceKind::Logs]
                .into_iter()
                .collect(),
            contents: Resources {
                logs: 5.0,
                ..Resources::default()
            },
        });
        colony.cats[0].carrying = Some(Carrying {
            kind: cat_sim::entities::CarryingKind::Logs,
            amount: 5.0,
            job_ended_at: 10_000,
            source_gather_spot: Some(format!("station-in|{building_id}|{transit_id}")),
        });
        colony.cats[0].destination = Some(Position {
            map: cat_sim::entities::MapType::World,
            x: 17.0,
            y: 18.0,
        });
        world.colonies.push(colony);

        save_world(&conn, &world).expect("save mid-haul");
        let restarted = load_world(&conn).expect("load").expect("persisted world");
        let colony = &restarted.colonies[0];
        assert_eq!(
            colony
                .stockpiles
                .iter()
                .find(|pile| pile.id == transit_id)
                .expect("transit store persisted")
                .contents
                .logs,
            5.0
        );
        assert_eq!(colony.cats[0].carrying.as_ref().unwrap().amount, 5.0);
        assert_eq!(
            colony.cats[0]
                .carrying
                .as_ref()
                .unwrap()
                .source_gather_spot
                .as_deref(),
            Some(format!("station-in|{building_id}|{transit_id}").as_str())
        );
        assert_eq!(colony.buildings.last().unwrap().production_progress, 317.5);
        assert_eq!(colony.buildings.last().unwrap().production_queue.len(), 1);
        assert!(!colony.buildings.last().unwrap().production_queue[0].repeat);
        assert!(colony.buildings.last().unwrap().production_paused);
    }

    #[test]
    fn legacy_mill_queue_is_initialized_once_and_player_cleared_empty_stays_empty() {
        let conn = Connection::open_in_memory().expect("memory db");
        init_schema(&conn).expect("schema");
        conn.execute(
            "INSERT INTO buildings (
                id, colonyId, type, level, position, constructionProgress,
                productionProgress, isComplete, productionQueue, productionPaused,
                productionQueueInitialized
             ) VALUES ('legacy-mill', 'colony-1', 'mill', 1, '{}', 100, 0, 1, '[]', 0, 0)",
            [],
        )
        .expect("legacy mill row");
        conn.execute(
            "INSERT INTO buildings (
                id, colonyId, type, level, position, constructionProgress,
                productionProgress, isComplete, productionQueue, productionPaused,
                productionQueueInitialized
             ) VALUES ('legacy-saw', 'colony-1', 'sawmill', 1, '{}', 100, 0, 1, '[]', 0, 0)",
            [],
        )
        .expect("legacy saw row");
        conn.execute_batch("ALTER TABLE buildings DROP COLUMN productionQueueInitialized;")
            .expect("simulate pre-Mill-queue schema");

        migrate_add_missing_columns(&conn).expect("one-time queue migration");
        let (mill_queue, initialized): (String, i64) = conn
            .query_row(
                "SELECT productionQueue, productionQueueInitialized
                 FROM buildings WHERE id = 'legacy-mill'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("migrated mill");
        assert_eq!(
            serde_json::from_str::<Vec<ProductionQueueEntry>>(&mill_queue).unwrap(),
            default_production_queue(BuildingType::Mill)
        );
        assert_eq!(initialized, 1);
        let saw_queue: String = conn
            .query_row(
                "SELECT productionQueue FROM buildings WHERE id = 'legacy-saw'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(saw_queue, "[]", "unrelated station queues are untouched");

        conn.execute(
            "UPDATE buildings SET productionQueue = '[]' WHERE id = 'legacy-mill'",
            [],
        )
        .expect("player clears queue");
        migrate_add_missing_columns(&conn).expect("idempotent restart");
        let cleared: String = conn
            .query_row(
                "SELECT productionQueue FROM buildings WHERE id = 'legacy-mill'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            cleared, "[]",
            "initialized empty queue remains player-owned"
        );
    }

    #[test]
    fn legacy_refiner_queues_initialize_once_and_player_cleared_empty_stays_empty() {
        let conn = Connection::open_in_memory().expect("memory db");
        init_schema(&conn).expect("schema");
        for (id, building_type) in [
            ("legacy-workshop", BuildingType::Workshop),
            ("legacy-smelter", BuildingType::Smelter),
        ] {
            conn.execute(
                "INSERT INTO buildings (
                    id, colonyId, type, level, position, constructionProgress,
                    productionProgress, isComplete, productionQueue, productionPaused,
                    productionQueueInitialized, physicalRefinerQueueInitialized
                 ) VALUES (?1, 'colony-1', ?2, 1, '{}', 100, 0, 1, '[]', 0, 1, 0)",
                params![id, building_type.as_str()],
            )
            .expect("legacy refiner row");
        }
        conn.execute_batch("ALTER TABLE buildings DROP COLUMN physicalRefinerQueueInitialized;")
            .expect("simulate pre-physical-refiner schema");

        migrate_add_missing_columns(&conn).expect("one-time refiner queue migration");
        for (id, building_type) in [
            ("legacy-workshop", BuildingType::Workshop),
            ("legacy-smelter", BuildingType::Smelter),
        ] {
            let (queue, initialized): (String, i64) = conn
                .query_row(
                    "SELECT productionQueue, physicalRefinerQueueInitialized
                     FROM buildings WHERE id = ?1",
                    [id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert_eq!(
                serde_json::from_str::<Vec<ProductionQueueEntry>>(&queue).unwrap(),
                default_production_queue(building_type)
            );
            assert_eq!(initialized, 1);
        }

        conn.execute(
            "UPDATE buildings SET productionQueue = '[]' WHERE id = 'legacy-workshop'",
            [],
        )
        .expect("player clears Workshop queue");
        migrate_add_missing_columns(&conn).expect("idempotent restart");
        let cleared: String = conn
            .query_row(
                "SELECT productionQueue FROM buildings WHERE id = 'legacy-workshop'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            cleared, "[]",
            "initialized empty queue remains player-owned"
        );
    }

    #[test]
    fn mill_local_inventories_and_both_haul_directions_resume_exactly_after_restart() {
        let conn = open_database(":memory:").expect("database");
        let mut world = new_world(6_414);
        let mut colony = found_colony(world.world_seed, "colony-1", 10_000, 6_414);
        let building_id = "restart-mill";
        colony.buildings.push(BuildingRuntime {
            id: building_id.to_owned(),
            building_type: BuildingType::Mill,
            position: TilePos { x: 18, y: 18 },
            is_complete: true,
            construction_progress: 100,
            production_progress: 317.5,
            assigned_cat: Some(colony.cats[0].id.clone()),
            production_queue: vec![ProductionQueueEntry {
                recipe_id: cat_sim::world_tick::MILL_RECIPE_ID.to_owned(),
                repeat: false,
            }],
            ..BuildingRuntime::default()
        });
        let rect = ZoneRect {
            x1: 18,
            y1: 18,
            x2: 20,
            y2: 20,
        };
        let input_id = cat_sim::stockpiles::station_input_id(building_id);
        let output_id = cat_sim::stockpiles::station_output_id(building_id);
        let transit_id = cat_sim::stockpiles::station_transit_id(building_id);
        for (id, accepts, contents) in [
            (
                input_id.clone(),
                [cat_sim::stockpiles::ResourceKind::Grain]
                    .into_iter()
                    .collect(),
                Resources {
                    grain: 4.0,
                    ..Resources::default()
                },
            ),
            (
                output_id.clone(),
                [cat_sim::stockpiles::ResourceKind::Flour]
                    .into_iter()
                    .collect(),
                Resources {
                    flour: 1.0,
                    ..Resources::default()
                },
            ),
            (
                transit_id.clone(),
                [cat_sim::stockpiles::ResourceKind::Flour]
                    .into_iter()
                    .collect(),
                Resources {
                    flour: 2.0,
                    ..Resources::default()
                },
            ),
        ] {
            colony.stockpiles.push(Stockpile {
                id,
                rect,
                accepts,
                contents,
            });
        }
        colony.resources.grain += 4.0;
        colony.resources.flour += 2.0;
        colony.cats[0].carrying = Some(Carrying {
            kind: cat_sim::entities::CarryingKind::Flour,
            amount: 2.0,
            job_ended_at: 10_000,
            source_gather_spot: Some(format!("station-in|{building_id}|{transit_id}")),
        });
        let general_id = colony
            .stockpiles
            .iter()
            .find(|pile| pile.is_general_storehouse())
            .unwrap()
            .id
            .clone();
        colony.cats[1].carrying = Some(Carrying {
            kind: cat_sim::entities::CarryingKind::Food,
            amount: 4.0,
            job_ended_at: 10_000,
            source_gather_spot: Some(format!("station-out|{building_id}|{general_id}")),
        });
        world.colonies.push(colony);

        save_world(&conn, &world).expect("save Mill mid-haul");
        let restarted = load_world(&conn).expect("load").expect("world");
        let colony = &restarted.colonies[0];
        assert_eq!(
            colony
                .buildings
                .iter()
                .find(|building| building.id == building_id)
                .unwrap()
                .production_progress,
            317.5
        );
        assert_eq!(
            colony
                .stockpiles
                .iter()
                .find(|pile| pile.id == input_id)
                .unwrap()
                .contents
                .grain,
            4.0
        );
        assert_eq!(
            colony
                .stockpiles
                .iter()
                .find(|pile| pile.id == output_id)
                .unwrap()
                .contents
                .flour,
            1.0
        );
        assert_eq!(colony.cats[0].carrying.as_ref().unwrap().amount, 2.0);
        assert_eq!(
            colony.cats[1]
                .carrying
                .as_ref()
                .unwrap()
                .source_gather_spot
                .as_deref(),
            Some(format!("station-out|{building_id}|{general_id}").as_str())
        );
    }

    #[test]
    fn physical_refiner_local_ledgers_queues_and_haul_markers_resume_exactly_after_restart() {
        let conn = open_database(":memory:").expect("database");
        let mut world = new_world(7_717);
        let mut colony = found_colony(world.world_seed, "colony-1", 10_000, 7_717);
        let general_id = colony
            .stockpiles
            .iter()
            .find(|pile| pile.is_general_storehouse())
            .unwrap()
            .id
            .clone();
        let workshop_id = "restart-workshop";
        let smelter_id = "restart-smelter";
        colony.buildings.extend([
            BuildingRuntime {
                id: workshop_id.to_owned(),
                building_type: BuildingType::Workshop,
                position: TilePos { x: 18, y: 18 },
                is_complete: true,
                construction_progress: 100,
                production_progress: 317.5,
                assigned_cat: Some(colony.cats[0].id.clone()),
                production_queue: vec![ProductionQueueEntry {
                    recipe_id: cat_sim::world_tick::WORKSHOP_RECIPE_ID.to_owned(),
                    repeat: false,
                }],
                production_paused: true,
                ..BuildingRuntime::default()
            },
            BuildingRuntime {
                id: smelter_id.to_owned(),
                building_type: BuildingType::Smelter,
                position: TilePos { x: 24, y: 18 },
                is_complete: true,
                construction_progress: 100,
                production_progress: 499.25,
                assigned_cat: Some(colony.cats[1].id.clone()),
                production_queue: vec![ProductionQueueEntry {
                    recipe_id: cat_sim::world_tick::SMELTER_RECIPE_ID.to_owned(),
                    repeat: true,
                }],
                ..BuildingRuntime::default()
            },
        ]);
        let rect = ZoneRect {
            x1: 18,
            y1: 18,
            x2: 20,
            y2: 20,
        };
        let workshop_input = cat_sim::stockpiles::station_input_id(workshop_id);
        let workshop_output = cat_sim::stockpiles::station_output_id(workshop_id);
        let workshop_transit = cat_sim::stockpiles::station_transit_id(workshop_id);
        let smelter_input = cat_sim::stockpiles::station_input_id(smelter_id);
        let smelter_output = cat_sim::stockpiles::station_output_id(smelter_id);
        let orphan_output = cat_sim::stockpiles::station_output_id("demolished-smelter");
        for (id, accepts, contents) in [
            (
                workshop_input.clone(),
                [cat_sim::stockpiles::ResourceKind::Materials]
                    .into_iter()
                    .collect(),
                Resources {
                    materials: 5.0,
                    ..Resources::default()
                },
            ),
            (
                workshop_output.clone(),
                [cat_sim::stockpiles::ResourceKind::Refined]
                    .into_iter()
                    .collect(),
                Resources {
                    refined: 1.0,
                    ..Resources::default()
                },
            ),
            (
                workshop_transit.clone(),
                [cat_sim::stockpiles::ResourceKind::Materials]
                    .into_iter()
                    .collect(),
                Resources {
                    materials: 2.0,
                    ..Resources::default()
                },
            ),
            (
                smelter_input.clone(),
                [cat_sim::stockpiles::ResourceKind::Ore]
                    .into_iter()
                    .collect(),
                Resources {
                    ore: 5.0,
                    ..Resources::default()
                },
            ),
            (
                smelter_output.clone(),
                [cat_sim::stockpiles::ResourceKind::Metal]
                    .into_iter()
                    .collect(),
                Resources {
                    metal: 1.0,
                    ..Resources::default()
                },
            ),
            (
                orphan_output.clone(),
                [cat_sim::stockpiles::ResourceKind::Metal]
                    .into_iter()
                    .collect(),
                Resources {
                    metal: 2.0,
                    ..Resources::default()
                },
            ),
        ] {
            colony.stockpiles.push(Stockpile {
                id,
                rect,
                accepts,
                contents,
            });
        }
        colony.cats[0].carrying = Some(Carrying {
            kind: CarryingKind::Materials,
            amount: 2.0,
            job_ended_at: 10_000,
            source_gather_spot: Some(format!("station-in|{workshop_id}|{workshop_transit}")),
        });
        colony.cats[1].carrying = Some(Carrying {
            kind: CarryingKind::Metal,
            amount: 1.0,
            job_ended_at: 10_000,
            source_gather_spot: Some(format!("station-out|{smelter_id}|{general_id}")),
        });
        world.colonies.push(colony);

        save_world(&conn, &world).expect("save refiner stages");
        let restarted = load_world(&conn).expect("load").expect("world");
        let colony = &restarted.colonies[0];
        for (id, kind, amount) in [
            (
                workshop_input.as_str(),
                cat_sim::stockpiles::ResourceKind::Materials,
                5.0,
            ),
            (
                workshop_output.as_str(),
                cat_sim::stockpiles::ResourceKind::Refined,
                1.0,
            ),
            (
                workshop_transit.as_str(),
                cat_sim::stockpiles::ResourceKind::Materials,
                2.0,
            ),
            (
                smelter_input.as_str(),
                cat_sim::stockpiles::ResourceKind::Ore,
                5.0,
            ),
            (
                smelter_output.as_str(),
                cat_sim::stockpiles::ResourceKind::Metal,
                1.0,
            ),
            (
                orphan_output.as_str(),
                cat_sim::stockpiles::ResourceKind::Metal,
                2.0,
            ),
        ] {
            let pile = colony
                .stockpiles
                .iter()
                .find(|pile| pile.id == id)
                .expect("local ledger persisted");
            assert_eq!(
                cat_sim::stockpiles::resource_amount(&pile.contents, kind),
                amount
            );
        }
        let workshop = colony
            .buildings
            .iter()
            .find(|building| building.id == workshop_id)
            .unwrap();
        assert_eq!(workshop.production_progress, 317.5);
        assert_eq!(
            workshop.production_queue,
            vec![ProductionQueueEntry {
                recipe_id: cat_sim::world_tick::WORKSHOP_RECIPE_ID.to_owned(),
                repeat: false,
            }]
        );
        assert!(workshop.production_paused);
        let smelter = colony
            .buildings
            .iter()
            .find(|building| building.id == smelter_id)
            .unwrap();
        assert_eq!(smelter.production_progress, 499.25);
        assert_eq!(
            smelter.production_queue,
            vec![ProductionQueueEntry {
                recipe_id: cat_sim::world_tick::SMELTER_RECIPE_ID.to_owned(),
                repeat: true,
            }]
        );
        assert_eq!(
            colony.cats[0]
                .carrying
                .as_ref()
                .unwrap()
                .source_gather_spot
                .as_deref(),
            Some(format!("station-in|{workshop_id}|{workshop_transit}").as_str())
        );
        assert_eq!(
            colony.cats[1]
                .carrying
                .as_ref()
                .unwrap()
                .source_gather_spot
                .as_deref(),
            Some(format!("station-out|{smelter_id}|{general_id}").as_str())
        );
    }

    #[test]
    fn offering_inbound_cargo_and_ritual_escrow_resume_exactly_after_restart() {
        let conn = open_database(":memory:").expect("database");
        let mut world = new_world(9_191);
        let mut colony = found_colony(world.world_seed, "colony-1", 10_000, 9_191);
        let source_id = colony
            .stockpiles
            .iter()
            .find(|pile| !pile.is_station_local())
            .expect("founded colony has visible storage")
            .id
            .clone();
        let inbound_escrow = cat_sim::stockpiles::station_input_id("offering-inbound");
        let ritual_escrow = cat_sim::stockpiles::station_input_id("offering-ritual");
        let carry_id = "restart-offering-carry";
        colony.jobs.push(JobRuntime {
            id: carry_id.to_owned(),
            kind: JobKind::CarryOffering,
            status: JobStatus::Active,
            requested_by: JobRequester::Player,
            assigned_cat: Some(colony.cats[0].id.clone()),
            duration_ms: 300_000,
            created_at: 10_000,
            metadata: JobMetadata::OfferingCarry {
                source_stockpile_id: source_id,
                site: Some(TilePos { x: 9, y: 11 }),
                accepted: false,
                escrow_id: inbound_escrow,
                delivered: 4.0,
            },
            ..JobRuntime::default()
        });
        colony.cats[0].carrying = Some(Carrying {
            kind: CarryingKind::Materials,
            amount: 6.0,
            job_ended_at: 10_001,
            source_gather_spot: Some(format!("offering-cargo:{carry_id}")),
        });
        colony.cats[0].destination = Some(Position {
            map: cat_sim::entities::MapType::World,
            x: f64::from(colony.anchor.x),
            y: f64::from(colony.anchor.y),
        });
        colony.stockpiles.push(Stockpile {
            id: ritual_escrow.clone(),
            rect: cat_sim::stockpiles::shrine_rect(colony.anchor.x, colony.anchor.y),
            accepts: [cat_sim::stockpiles::ResourceKind::Materials]
                .into_iter()
                .collect(),
            contents: Resources {
                materials: 10.0,
                ..Resources::default()
            },
        });
        colony.resources.materials += 10.0;
        colony.jobs.push(JobRuntime {
            id: "restart-offering-ritual".to_owned(),
            kind: JobKind::PerformOffering,
            status: JobStatus::Active,
            requested_by: JobRequester::Leader,
            assigned_cat: Some(colony.cats[1].id.clone()),
            duration_ms: 2_400_000,
            created_at: 10_000,
            started_at: Some(10_000),
            ends_at: Some(2_410_000),
            metadata: JobMetadata::OfferingRitual {
                escrow_id: ritual_escrow,
                amount: 10.0,
            },
            ..JobRuntime::default()
        });
        world.colonies.push(colony);

        save_world(&conn, &world).expect("save both offering stages");
        let mut restarted = load_world(&conn)
            .expect("load both offering stages")
            .expect("persisted world");

        assert_eq!(restarted.colonies[0], world.colonies[0]);

        for now_ms in [11_000, 12_000, 13_000] {
            assert_eq!(
                world_tick(&mut restarted, now_ms),
                world_tick(&mut world, now_ms),
                "restarted physical offerings must make the same next-tick decisions"
            );
            assert_eq!(restarted, world);
        }
    }

    #[test]
    fn restart_at_organic_arrival_preserves_the_exact_deadline_outcome() {
        const STARTED_AT: i64 = 10_000;
        const STEP_MS: i64 = 15 * 60_000;
        const MAX_ARRIVAL_HOUR: i64 = 60;

        let conn = open_database(":memory:").expect("database");
        let seed = 42;
        let mut uninterrupted = new_world(seed);
        let mut colony = found_colony(seed, "colony-1", STARTED_AT, seed);
        // The restart assertion is about migration deadlines, not the strict
        // manual opening. Give this unattended persistence fixture the real
        // Farmer prerequisite that owns its food/water loop.
        establish_persistence_campaign_core(&mut colony);
        uninterrupted.colonies.push(colony);
        let mut now = STARTED_AT;
        while now < STARTED_AT + MAX_ARRIVAL_HOUR * 3_600_000
            && uninterrupted.colonies[0]
                .migration_state
                .probationary_migrants
                .is_empty()
        {
            now += STEP_MS;
            assert_eq!(world_tick(&mut uninterrupted, now)[0].reset_reason, None);
        }
        let arrival_ids = uninterrupted.colonies[0]
            .migration_state
            .probationary_migrants
            .iter()
            .map(|migrant| migrant.id.clone())
            .collect::<Vec<_>>();
        assert!(
            !arrival_ids.is_empty(),
            "fixture never reached organic migration"
        );
        let arrival = uninterrupted.colonies[0]
            .migration_state
            .probationary_migrants
            .first()
            .expect("organic migrant");
        assert_eq!(
            arrival.housing_deadline_game_minute - arrival.arrived_game_minute,
            36 * 60
        );
        let deadline_at = STARTED_AT
            + i64::try_from(arrival.housing_deadline_game_minute)
                .expect("bounded campaign deadline")
                * 60_000;

        save_world(&conn, &uninterrupted).expect("save at arrival");
        let mut restarted = load_world(&conn)
            .expect("load at arrival")
            .expect("persisted world");
        assert_eq!(
            restarted.colonies[0].migration_state,
            uninterrupted.colonies[0].migration_state
        );
        assert_eq!(restarted.colonies[0].migration_departures, 0);
        for id in &arrival_ids {
            assert!(restarted.colonies[0].cats.iter().any(|cat| cat.id == *id));
        }

        while now < deadline_at {
            now += STEP_MS;
            // Hold the deliberately poor branch below the materials bar so no later
            // cohort obscures the first cohort's exact deadline comparison.
            for world in [&mut uninterrupted, &mut restarted] {
                world.colonies[0].resources.materials = 0.0;
                world.colonies[0].resources.planks = 0.0;
                world.colonies[0].resources.blocks = 0.0;
            }
            assert_eq!(
                world_tick(&mut uninterrupted, now),
                world_tick(&mut restarted, now),
                "restart changed tick outcome at {now}"
            );
            assert_eq!(
                restarted.colonies[0].migration_state, uninterrupted.colonies[0].migration_state,
                "restart changed probation state at {now}"
            );
            assert_eq!(
                restarted.colonies[0].migration_departures,
                uninterrupted.colonies[0].migration_departures,
                "restart changed departure count at {now}"
            );
        }

        for world in [&uninterrupted, &restarted] {
            let colony = &world.colonies[0];
            assert!(arrival_ids.iter().all(|id| {
                !colony.cats.iter().any(|cat| cat.id == *id)
                    && !colony
                        .migration_state
                        .probationary_migrants
                        .iter()
                        .any(|migrant| migrant.id == *id)
            }));
            assert_eq!(colony.migration_departures, arrival_ids.len() as u64);
        }
    }

    #[test]
    fn low_comfort_restart_preserves_manual_scholar_and_releases_role_owned_scholar() {
        let conn = open_database(":memory:").expect("database");
        let mut world = new_world(77);
        let mut colony = found_colony(77, "colony-1", 10_000, 77);
        colony.jobs.clear();
        colony.resources.food = 1.0;
        colony.resources.water = 1.0;
        colony.revealed_tiles.insert(TilePos { x: 99, y: 99 });
        let automated_scholar = colony.cats[0].id.clone();
        let manual_scholar = colony.cats[1].id.clone();
        colony.buildings.push(BuildingRuntime {
            id: "auto-research".to_owned(),
            building_type: BuildingType::ResearchHut,
            position: TilePos { x: 20, y: 20 },
            is_complete: true,
            construction_progress: 100,
            assigned_cat: Some(automated_scholar),
            automated_by: Some(OfficerRole::Loremaster),
            ..BuildingRuntime::default()
        });
        colony.buildings.push(BuildingRuntime {
            id: "manual-school".to_owned(),
            building_type: BuildingType::School,
            position: TilePos { x: 24, y: 20 },
            is_complete: true,
            construction_progress: 100,
            assigned_cat: Some(manual_scholar.clone()),
            automated_by: None,
            ..BuildingRuntime::default()
        });
        world.colonies.push(colony);
        save_world(&conn, &world).expect("save before restart");
        let mut restarted = load_world(&conn)
            .expect("load after restart")
            .expect("saved world exists");

        let _ = world_tick(&mut restarted, 70_000);

        let automated = restarted.colonies[0]
            .buildings
            .iter()
            .find(|building| building.id == "auto-research")
            .expect("role-owned hut persists");
        assert_eq!(automated.assigned_cat, None);
        assert_eq!(automated.automated_by, None);
        let manual = restarted.colonies[0]
            .buildings
            .iter()
            .find(|building| building.id == "manual-school")
            .expect("manual school persists");
        assert_eq!(
            manual.assigned_cat.as_deref(),
            Some(manual_scholar.as_str())
        );
        assert_eq!(manual.automated_by, None);
    }

    #[test]
    fn knowledge_blind_mid_search_destination_and_provisional_notes_survive_restart() {
        let conn = open_database(":memory:").expect("database");
        let mut world = new_world(4_242);
        let mut colony = found_colony(world.world_seed, "colony-1", 10_000, 4_242);
        let scout_id = colony.cats[0].id.clone();
        let far = TilePos { x: 30, y: 31 };
        colony.revealed_tiles.insert(far);
        let provisional: BTreeSet<_> = [TilePos { x: 40, y: 41 }].into_iter().collect();
        colony
            .provisional_tiles
            .insert(scout_id.clone(), provisional.clone());
        colony.jobs.push(JobRuntime {
            id: "scout-persist".to_owned(),
            kind: JobKind::Explore,
            status: JobStatus::Active,
            requested_by: JobRequester::Player,
            assigned_cat: Some(scout_id.clone()),
            duration_ms: 60_000,
            speed: 1.0,
            yield_amount: 1.0,
            click_count: 0,
            created_at: 11_000,
            started_at: Some(11_000),
            ends_at: Some(71_000),
            completed_at: None,
            metadata: JobMetadata::Scout {
                mission: ScoutMission::Resource(ScoutResource::Wood),
                // No resource has been physically observed yet. A restart must
                // preserve this distinction instead of resolving a hidden target.
                target: None,
                destination: Some(TilePos { x: 21, y: 19 }),
                accepted: true,
                found: false,
            },
        });
        world.colonies.push(colony);

        save_world(&conn, &world).expect("save");
        let loaded = load_world(&conn).expect("load").expect("world");
        let loaded = &loaded.colonies[0];

        assert!(loaded.revealed_tiles.contains(&far));
        assert_eq!(loaded.provisional_tiles.get(&scout_id), Some(&provisional));
        assert!(matches!(
            loaded.jobs.last().map(|job| &job.metadata),
            Some(JobMetadata::Scout {
                mission: ScoutMission::Resource(ScoutResource::Wood),
                target: None,
                destination: Some(TilePos { x: 21, y: 19 }),
                accepted: true,
                found: false,
            })
        ));
    }

    #[test]
    fn legacy_null_revealed_tiles_restores_exact_founding_knowledge() {
        let conn = open_database(":memory:").expect("database");
        let mut world = new_world(4_242);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 10_000, 4_242));
        save_world(&conn, &world).expect("save");
        conn.execute(
            "UPDATE colonies SET revealedTiles = NULL WHERE id = 'colony-1'",
            [],
        )
        .expect("simulate legacy row");

        let loaded = load_world(&conn).expect("load").expect("world");
        let colony = &loaded.colonies[0];
        assert_eq!(
            colony.revealed_tiles,
            founding_revealed_tiles(colony.anchor, &colony.claimed_tiles)
        );
        assert!(
            colony
                .claimed_tiles
                .iter()
                .all(|tile| colony.revealed_tiles.contains(tile))
        );
    }

    #[test]
    fn legacy_null_provisional_tiles_loads_an_empty_scout_notebook() {
        let conn = open_database(":memory:").expect("database");
        let mut world = new_world(4_242);
        let mut colony = found_colony(world.world_seed, "colony-1", 10_000, 4_242);
        colony.provisional_tiles.insert(
            colony.cats[0].id.clone(),
            [TilePos { x: 40, y: 41 }].into_iter().collect(),
        );
        world.colonies.push(colony);
        save_world(&conn, &world).expect("save");
        conn.execute(
            "UPDATE colonies SET provisionalTiles = NULL WHERE id = 'colony-1'",
            [],
        )
        .expect("simulate legacy row");

        let loaded = load_world(&conn).expect("load").expect("world");
        assert!(loaded.colonies[0].provisional_tiles.is_empty());
    }

    #[test]
    fn init_schema_backfills_post_p12_columns_on_a_legacy_database() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        // Simulate a DB created before P12: colonies/cats tables exist but lack the
        // officers/stockpiles/stockLedger/skills columns. CREATE TABLE IF NOT EXISTS
        // in init_schema must NOT recreate them; the migration must add the columns.
        conn.execute_batch(
            "CREATE TABLE colonies (id TEXT PRIMARY KEY, name TEXT NOT NULL,
                 status TEXT NOT NULL, resources TEXT NOT NULL, gridSize INTEGER NOT NULL,
                 createdAt INTEGER NOT NULL, lastTick INTEGER NOT NULL,
                 lastAttack INTEGER NOT NULL, isPregnant INTEGER);
             CREATE TABLE cats (id TEXT PRIMARY KEY, colonyId TEXT NOT NULL,
                 name TEXT NOT NULL);
             CREATE TABLE buildings (id TEXT NOT NULL, colonyId TEXT NOT NULL,
                 assignedCatId TEXT);",
        )
        .expect("legacy tables");

        for (table, column) in [
            ("colonies", "upgradeLevels"),
            ("colonies", "officers"),
            ("colonies", "stockpiles"),
            ("colonies", "farms"),
            ("colonies", "stockLedger"),
            ("colonies", "provisionalTiles"),
            ("colonies", "coin"),
            ("colonies", "migrationState"),
            ("colonies", "migrationDepartures"),
            ("colonies", "isGlobal"),
            ("colonies", "foundingScale"),
            ("colonies", "ownerPlayerId"),
            ("colonies", "knownVillageIds"),
            ("colonies", "villageTradeOffers"),
            ("colonies", "lastLoremasterUnlockAt"),
            ("colonies", "lastTitheAt"),
            ("colonies", "lastOfferingAt"),
            ("colonies", "recipeEntitlementRulesVersion"),
            ("cats", "skills"),
            ("cats", "boosted"),
            ("buildings", "automatedOfficerRole"),
            ("buildings", "productionQueueInitialized"),
            ("buildings", "physicalRefinerQueueInitialized"),
        ] {
            assert!(!column_exists(&conn, table, column).unwrap());
        }

        init_schema(&conn).expect("init schema migrates the legacy tables");

        for (table, column) in [
            ("colonies", "upgradeLevels"),
            ("colonies", "officers"),
            ("colonies", "stockpiles"),
            ("colonies", "farms"),
            ("colonies", "stockLedger"),
            ("colonies", "provisionalTiles"),
            ("colonies", "coin"),
            ("colonies", "migrationState"),
            ("colonies", "migrationDepartures"),
            ("colonies", "isGlobal"),
            ("colonies", "foundingScale"),
            ("colonies", "ownerPlayerId"),
            ("colonies", "knownVillageIds"),
            ("colonies", "villageTradeOffers"),
            ("colonies", "lastLoremasterUnlockAt"),
            ("colonies", "lastTitheAt"),
            ("colonies", "lastOfferingAt"),
            ("colonies", "recipeEntitlementRulesVersion"),
            ("cats", "skills"),
            ("cats", "boosted"),
            ("buildings", "automatedOfficerRole"),
            ("buildings", "productionQueueInitialized"),
            ("buildings", "physicalRefinerQueueInitialized"),
        ] {
            assert!(
                column_exists(&conn, table, column).unwrap(),
                "{table}.{column} should be back-filled"
            );
        }

        // Idempotent: running again does not error (columns already present).
        init_schema(&conn).expect("re-running init_schema is a no-op");
    }

    #[test]
    fn legacy_database_without_upgrade_levels_migrates_and_round_trips_world() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("create current schema");
        conn.execute_batch("ALTER TABLE colonies DROP COLUMN upgradeLevels;")
            .expect("simulate the pre-upgradeLevels schema");
        assert!(!column_exists(&conn, "colonies", "upgradeLevels").unwrap());

        init_schema(&conn).expect("backfill upgradeLevels");
        let mut world = new_world(42);
        let mut colony = found_colony(42, "legacy", 1_000_000, 9);
        colony.upgrade_levels.click_power = 3;
        world.colonies.push(colony);
        save_world(&conn, &world).expect("save migrated world");
        let loaded = load_world(&conn)
            .expect("load migrated world")
            .expect("saved world exists");
        assert_eq!(loaded, world);
    }

    #[test]
    fn recipe_entitlement_version_queue_ownership_and_leader_timestamp_survive_restart() {
        let conn = open_database(":memory:").expect("database");
        let mut world = new_world(42);
        let mut colony = found_colony(42, "entitlement", 1_000_000, 42);
        colony.upgrade_tree.owned_node_ids.extend(
            [
                "carpentry_preparation",
                "textiles",
                "weaponsmithing",
                "armorsmithing",
            ]
            .map(str::to_owned),
        );
        colony.last_leader_research_choice_at = Some(1_234_567);
        colony.buildings.push(BuildingRuntime {
            id: "persisted-sawmill".to_owned(),
            building_type: BuildingType::Sawmill,
            is_complete: true,
            construction_progress: 100,
            production_queue: vec![ProductionQueueEntry {
                recipe_id: cat_sim::world_tick::SAWMILL_RECIPE_ID.to_owned(),
                repeat: false,
            }],
            ..BuildingRuntime::default()
        });
        world.colonies.push(colony);
        save_world(&conn, &world).expect("save");

        let loaded = load_world(&conn).expect("load").expect("world");
        let colony = &loaded.colonies[0];
        assert_eq!(
            colony.recipe_entitlement_rules_version,
            cat_sim::world_tick::CURRENT_RECIPE_ENTITLEMENT_RULES_VERSION
        );
        for study in [
            "carpentry_preparation",
            "textiles",
            "weaponsmithing",
            "armorsmithing",
        ] {
            assert!(
                colony
                    .upgrade_tree
                    .owned_node_ids
                    .iter()
                    .any(|id| id == study)
            );
        }
        for recipe_id in [
            cat_sim::world_tick::CLOTHIER_RECIPE_ID,
            cat_sim::world_tick::TANNERY_RECIPE_ID,
            cat_sim::world_tick::SMITHY_WEAPON_RECIPE_ID,
            cat_sim::world_tick::SMITHY_ARMOR_RECIPE_ID,
        ] {
            assert!(
                cat_sim::world_tick::catalog_recipe_entitlement(colony, recipe_id).available,
                "persisted study did not restore {recipe_id}"
            );
        }
        assert_eq!(colony.last_leader_research_choice_at, Some(1_234_567));
        assert_eq!(
            colony
                .buildings
                .iter()
                .find(|building| building.id == "persisted-sawmill")
                .unwrap()
                .production_queue,
            [ProductionQueueEntry {
                recipe_id: cat_sim::world_tick::SAWMILL_RECIPE_ID.to_owned(),
                repeat: false,
            }]
        );
    }

    #[test]
    fn legacy_daily_research_column_preserves_the_leader_boundary_across_restarts() {
        const DAY_MS: i64 = 24 * 60 * 60 * 1_000;
        let conn = open_database(":memory:").expect("database");
        let chosen_at = 100_000;
        let boundary = chosen_at + DAY_MS;
        let mut world = new_world(79);
        let mut colony = found_colony(79, "daily-research", chosen_at, 79);
        colony.leader_id = Some(colony.cats[0].id.clone());
        colony.upgrade_tree.research_points = 100.0;
        colony.last_leader_research_choice_at = Some(chosen_at);
        world.colonies.push(colony);
        save_world(&conn, &world).expect("save at T");

        let stored: Option<i64> = conn
            .query_row(
                "SELECT lastLoremasterUnlockAt FROM colonies WHERE id = 'daily-research'",
                [],
                |row| row.get(0),
            )
            .expect("read legacy compatibility column");
        assert_eq!(stored, Some(chosen_at));

        let mut before = load_world(&conn).unwrap().unwrap();
        let _ = world_tick(&mut before, boundary - 1);
        assert!(before.colonies[0].upgrade_tree.owned_node_ids.is_empty());
        assert_eq!(
            before.colonies[0].last_leader_research_choice_at,
            Some(chosen_at)
        );

        let mut exact = load_world(&conn).unwrap().unwrap();
        let _ = world_tick(&mut exact, boundary);
        assert_eq!(
            exact.colonies[0].upgrade_tree.owned_node_ids,
            ["research_hut"]
        );
        assert_eq!(
            exact.colonies[0].last_leader_research_choice_at,
            Some(boundary)
        );
        save_world(&conn, &exact).expect("save exact-boundary result");

        let mut immediate = load_world(&conn).unwrap().unwrap();
        let _ = world_tick(&mut immediate, boundary + 1_000);
        assert_eq!(
            immediate.colonies[0].upgrade_tree.owned_node_ids,
            ["research_hut"]
        );
        assert_eq!(
            immediate.colonies[0].last_leader_research_choice_at,
            Some(boundary)
        );
    }

    #[test]
    fn missing_recipe_entitlement_version_migrates_to_legacy_grandfathering() {
        let conn = open_database(":memory:").expect("database");
        let mut world = new_world(42);
        let mut colony = found_colony(42, "legacy-entitlement", 1_000_000, 42);
        colony.buildings.push(BuildingRuntime {
            id: "legacy-sawmill".to_owned(),
            building_type: BuildingType::Sawmill,
            is_complete: true,
            construction_progress: 100,
            production_queue: vec![ProductionQueueEntry {
                recipe_id: cat_sim::world_tick::SAWMILL_RECIPE_ID.to_owned(),
                repeat: true,
            }],
            ..BuildingRuntime::default()
        });
        world.colonies.push(colony);
        save_world(&conn, &world).expect("save current row");
        conn.execute_batch("ALTER TABLE colonies DROP COLUMN recipeEntitlementRulesVersion;")
            .expect("simulate old schema");
        init_schema(&conn).expect("migrate missing version");

        let loaded = load_world(&conn).expect("load").expect("world");
        let colony = &loaded.colonies[0];
        assert_eq!(colony.recipe_entitlement_rules_version, 0);
        assert!(colony.upgrade_tree.owned_node_ids.is_empty());
        for recipe_id in [
            cat_sim::world_tick::CLOTHIER_RECIPE_ID,
            cat_sim::world_tick::TANNERY_RECIPE_ID,
            cat_sim::world_tick::SMITHY_WEAPON_RECIPE_ID,
            cat_sim::world_tick::SMITHY_ARMOR_RECIPE_ID,
        ] {
            assert!(
                cat_sim::world_tick::catalog_recipe_entitlement(colony, recipe_id).available,
                "rules-v0 restart did not grandfather {recipe_id}"
            );
        }
        let sawmill = colony
            .buildings
            .iter()
            .find(|building| building.id == "legacy-sawmill")
            .unwrap();
        assert_eq!(
            sawmill.production_queue,
            [ProductionQueueEntry {
                recipe_id: cat_sim::world_tick::SAWMILL_RECIPE_ID.to_owned(),
                repeat: true,
            }]
        );
        assert!(
            cat_sim::world_tick::production_recipe_availability(
                colony,
                BuildingType::Sawmill,
                cat_sim::world_tick::SAWMILL_RECIPE_ID
            )
            .is_some_and(|recipe| recipe.available)
        );
    }

    #[test]
    fn malformed_recipe_entitlement_versions_fall_back_to_legacy_grandfathering() {
        for malformed in [-1_i64, i64::from(u32::MAX) + 1] {
            let conn = open_database(":memory:").expect("database");
            let mut world = new_world(42);
            world
                .colonies
                .push(found_colony(42, "malformed-entitlement", 1_000_000, 42));
            save_world(&conn, &world).expect("save current row");
            conn.execute(
                "UPDATE colonies SET recipeEntitlementRulesVersion = ?1",
                [malformed],
            )
            .expect("inject malformed version");

            let loaded = load_world(&conn).expect("load").expect("world");
            let colony = &loaded.colonies[0];
            assert_eq!(colony.recipe_entitlement_rules_version, 0, "{malformed}");
            assert!(
                cat_sim::world_tick::production_recipe_availability(
                    colony,
                    BuildingType::Sawmill,
                    cat_sim::world_tick::SAWMILL_RECIPE_ID,
                )
                .is_some_and(|recipe| recipe.available),
                "malformed version {malformed} must retain legacy production"
            );
        }
    }

    #[test]
    fn legacy_building_assignments_are_released_once_when_provenance_is_added() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("create current schema");
        let mut world = new_world(42);
        let mut colony = found_colony(42, "legacy-workers", 1_000_000, 42);
        let worker = colony.cats[0].id.clone();
        colony.buildings[0].assigned_cat = Some(worker.clone());
        colony.buildings[0].automated_by = None;
        world.colonies.push(colony);
        save_world(&conn, &world).expect("save pre-provenance world");

        conn.execute_batch("ALTER TABLE buildings DROP COLUMN automatedOfficerRole;")
            .expect("simulate legacy buildings table");
        init_schema(&conn).expect("add worker provenance");
        let mut migrated = load_world(&conn)
            .expect("load migrated world")
            .expect("saved world exists");
        assert_eq!(migrated.colonies[0].buildings[0].assigned_cat, None);

        // Once the column exists, NULL is a real manual assignment and the
        // idempotent migration must never clear it on subsequent restarts.
        migrated.colonies[0].buildings[0].assigned_cat = Some(worker.clone());
        migrated.colonies[0].buildings[0].automated_by = None;
        save_world(&conn, &migrated).expect("save explicit manual assignment");
        init_schema(&conn).expect("idempotent schema init");
        let restarted = load_world(&conn)
            .expect("reload manual assignment")
            .expect("saved world exists");
        assert_eq!(
            restarted.colonies[0].buildings[0].assigned_cat.as_deref(),
            Some(worker.as_str())
        );
        assert_eq!(restarted.colonies[0].buildings[0].automated_by, None);
    }

    #[test]
    fn save_world_load_world_round_trips_colony_resources_cats_and_jobs() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");

        let mut world = new_world(20_240_703);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000_000, 42));
        world.colonies[0].resources.food = 123.5;
        world.colonies[0].resources.fish = 8.75;
        world.colonies[0].resources.water = 87.25;
        // P16/P19 clothing chain slice: the new resource fields must round-trip too —
        // `resources` persists as a single JSON blob column, so this exercises the
        // Resources serde shape end-to-end (not just the in-memory struct).
        world.colonies[0].resources.fibre = 6.0;
        world.colonies[0].resources.hide = 4.5;
        world.colonies[0].resources.cloth = 2.5;
        world.colonies[0].resources.leather = 1.5;
        world.colonies[0].resources.grain = 9.0;
        world.colonies[0].resources.logs = 10.0;

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
                colony_id: "colony-1".to_owned(),
                now_ms: 1_000_000,
            },
        );
        assert!(result.ok, "{result:?}");

        // P12.1/P12.2/P12.3 state must survive the round trip.
        let officer_cat = world.colonies[0].cats[0].id.clone();
        world.colonies[0].cats[0].gain_skill(cat_sim::skills::Labor::Hunt, 3.0);
        // P15 cat booster: the boosted flag must persist across a save/load cycle.
        world.colonies[0].cats[0].boosted = true;
        world.colonies[0]
            .officers
            .insert(cat_sim::officers::OfficerRole::Captain, officer_cat);
        // A designated stockpile (the shrine reservoir was seeded at founding).
        world.colonies[0]
            .stockpiles
            .push(cat_sim::stockpiles::Stockpile {
                id: "stockpile-a".to_owned(),
                rect: cat_sim::zones::ZoneRect {
                    x1: 8,
                    y1: 8,
                    x2: 9,
                    y2: 9,
                },
                accepts: [cat_sim::stockpiles::ResourceKind::Food]
                    .into_iter()
                    .collect(),
                contents: cat_sim::entities::Resources::default(),
            });
        // P16: a gather spot (its own pile + bookkeeping record) and an in-flight mover
        // job referencing it (`JobMetadata::GatherHaul`) must all survive the round trip.
        world.colonies[0]
            .stockpiles
            .push(cat_sim::stockpiles::Stockpile {
                id: "gather-1".to_owned(),
                rect: cat_sim::zones::ZoneRect {
                    x1: 30,
                    y1: 30,
                    x2: 30,
                    y2: 30,
                },
                accepts: [cat_sim::stockpiles::ResourceKind::Water]
                    .into_iter()
                    .collect(),
                contents: cat_sim::entities::Resources::default(),
            });
        world.colonies[0]
            .gather_spots
            .push(cat_sim::stockpiles::GatherSpot {
                stockpile_id: "gather-1".to_owned(),
                kind: cat_sim::stockpiles::ResourceKind::Water,
                expires_at_ms: 1_500_000,
                purpose: cat_sim::stockpiles::GatherSpotPurpose::General,
            });
        let farm_worker_id = world.colonies[0].cats[1].id.clone();
        world.colonies[0].farms.push(FarmPlot {
            id: "farm-a".to_owned(),
            rect: ZoneRect {
                x1: 10,
                y1: 10,
                x2: 11,
                y2: 11,
            },
            crop: cat_sim::farming::CropKind::Grain,
            planted_at: 1_100_000,
            stage: cat_sim::farming::FarmStage::Growing,
            worker_id: Some(farm_worker_id),
            work_phase: cat_sim::farming::FarmWorkPhase::Tending,
            pending_output: 0.0,
            growth_hours: 7.5,
            fertility: 1.25,
        });
        let farm_carrier_id = world.colonies[0].cats[2].id.clone();
        let farm_gather_id = "farm-gather:farm-hauling".to_owned();
        world.colonies[0]
            .stockpiles
            .push(cat_sim::stockpiles::Stockpile {
                id: farm_gather_id.clone(),
                rect: ZoneRect {
                    x1: 13,
                    y1: 10,
                    x2: 13,
                    y2: 10,
                },
                accepts: [cat_sim::stockpiles::ResourceKind::Grain]
                    .into_iter()
                    .collect(),
                contents: Resources::default(),
            });
        world.colonies[0]
            .gather_spots
            .push(cat_sim::stockpiles::GatherSpot {
                stockpile_id: farm_gather_id.clone(),
                kind: cat_sim::stockpiles::ResourceKind::Grain,
                expires_at_ms: i64::MAX,
                purpose: cat_sim::stockpiles::GatherSpotPurpose::General,
            });
        world.colonies[0].farms.push(FarmPlot {
            id: "farm-hauling".to_owned(),
            rect: ZoneRect {
                x1: 12,
                y1: 10,
                x2: 12,
                y2: 10,
            },
            crop: cat_sim::farming::CropKind::Grain,
            planted_at: 1_050_000,
            stage: cat_sim::farming::FarmStage::Soil,
            growth_hours: 0.0,
            fertility: 0.8,
            worker_id: Some(farm_carrier_id.clone()),
            work_phase: cat_sim::farming::FarmWorkPhase::Hauling,
            pending_output: 1.0,
        });
        world.colonies[0].cats[2].current_task = Some(TaskType::Farm);
        world.colonies[0].cats[2].carrying = Some(Carrying {
            kind: CarryingKind::Grain,
            amount: 2.0,
            job_ended_at: 1_100_000,
            source_gather_spot: Some(format!("farm-out|farm-hauling|{farm_gather_id}")),
        });
        world.colonies[0].cats[2].activity = CatActivity::Traveling;
        let mover_cat_id = world.colonies[0].cats[0].id.clone();
        world.colonies[0].jobs.push(JobRuntime {
            id: "job-mover".to_owned(),
            kind: JobKind::HaulGatherSpot,
            status: JobStatus::Active,
            assigned_cat: Some(mover_cat_id),
            metadata: JobMetadata::GatherHaul {
                stockpile_id: "gather-1".to_owned(),
                site: Some(TilePos { x: 30, y: 30 }),
                accepted: true,
            },
            ..JobRuntime::default()
        });
        let fishing_site = {
            let seed = world.world_seed;
            let colony = &world.colonies[0];
            colony
                .world_tiles
                .keys()
                .copied()
                .find(|site| {
                    let water = TilePos {
                        x: site.x,
                        y: site.y - 1,
                    };
                    if !colony.revealed_tiles.contains(site)
                        || !colony.world_tiles.contains_key(&water)
                        || cat_sim::world_tick::stockpile_placement_error(
                            colony,
                            ZoneRect {
                                x1: site.x,
                                y1: site.y,
                                x2: site.x,
                                y2: site.y,
                            },
                            seed,
                            false,
                        )
                        .is_some()
                    {
                        return false;
                    }
                    let mut projected = colony.clone();
                    projected.revealed_tiles.insert(water);
                    let tile = projected.world_tiles.get_mut(&water).unwrap();
                    tile.tile_type = cat_sim::types::TileType::River;
                    tile.resources.water = 100;
                    cat_sim::world_tick::is_reachable_fishing_shore(&projected, *site, seed)
                })
                .expect("round-trip fixture has a reachable clear bank")
        };
        let fishing_water = TilePos {
            x: fishing_site.x,
            y: fishing_site.y - 1,
        };
        world.colonies[0].revealed_tiles.insert(fishing_water);
        let water_tile = world.colonies[0]
            .world_tiles
            .get_mut(&fishing_water)
            .unwrap();
        water_tile.tile_type = cat_sim::types::TileType::River;
        water_tile.resources.water = 100;
        world.colonies[0]
            .stockpiles
            .push(cat_sim::stockpiles::Stockpile {
                id: "fishing-shore-1".to_owned(),
                rect: ZoneRect {
                    x1: fishing_site.x,
                    y1: fishing_site.y,
                    x2: fishing_site.x,
                    y2: fishing_site.y,
                },
                accepts: [cat_sim::stockpiles::ResourceKind::Fish]
                    .into_iter()
                    .collect(),
                contents: cat_sim::entities::Resources::default(),
            });
        world.colonies[0]
            .gather_spots
            .push(cat_sim::stockpiles::GatherSpot {
                stockpile_id: "fishing-shore-1".to_owned(),
                kind: cat_sim::stockpiles::ResourceKind::Fish,
                expires_at_ms: i64::MAX,
                purpose: cat_sim::stockpiles::GatherSpotPurpose::Fishing,
            });
        world.colonies[0].fish_habitats.insert(
            fishing_water,
            cat_sim::stockpiles::FishPopulation {
                stock: 7.25,
                capacity: cat_sim::stockpiles::FISH_POPULATION_CAPACITY,
                last_replenished_at_ms: 1_234_567,
            },
        );
        let fisher_id = world.colonies[0].cats[1].id.clone();
        world.colonies[0].jobs.push(JobRuntime {
            id: "job-fishing-restart".to_owned(),
            kind: JobKind::Fish,
            status: JobStatus::Active,
            assigned_cat: Some(fisher_id),
            created_at: 900_000,
            started_at: Some(900_000),
            ends_at: Some(3_600_000),
            duration_ms: 2_700_000,
            metadata: JobMetadata::Hauling {
                site: Some(fishing_site),
                total_yield: None,
                trips_done: 0,
                next_trip_at: None,
                accepted: true,
            },
            ..JobRuntime::default()
        });

        // Physical Accountant JSON is durable inside the existing stockLedger column: a
        // restart during the pile dwell must not silently return the cat to its desk or make
        // every pile fresh.
        world.colonies[0].stock_ledger = StockLedger::counted_with_piles(
            &world.colonies[0].resources,
            &world.colonies[0].stockpiles,
            1_000_000,
        );
        world.colonies[0].stock_ledger.steward_managed_piles.insert(
            "stockpile-a".to_owned(),
            cat_sim::ledger::StewardManagedPile {
                station_id: "mill-restart".to_owned(),
                resource: cat_sim::stockpiles::ResourceKind::Food,
                active: false,
            },
        );
        let steward_transit_id = cat_sim::stockpiles::station_transit_id("steward:job-balance");
        world.colonies[0]
            .stockpiles
            .push(cat_sim::stockpiles::make_station_store(
                steward_transit_id.clone(),
                ZoneRect {
                    x1: 8,
                    y1: 8,
                    x2: 8,
                    y2: 8,
                },
                [cat_sim::stockpiles::ResourceKind::Food],
            ));
        world.colonies[0]
            .stockpiles
            .last_mut()
            .expect("transit inserted")
            .contents
            .food = 3.0;
        world.colonies[0].jobs.push(JobRuntime {
            id: "job-balance".to_owned(),
            kind: JobKind::HaulGatherSpot,
            status: JobStatus::Cancelled,
            requested_by: cat_sim::world_tick::JobRequester::System,
            metadata: JobMetadata::StockpileHaul {
                source_stockpile_id: "stockpile-a".to_owned(),
                destination_stockpile_id: cat_sim::stockpiles::GENERAL_STOREHOUSE_ID.to_owned(),
                kind: cat_sim::stockpiles::ResourceKind::Food,
                site: Some(TilePos { x: 8, y: 8 }),
                accepted: true,
                transit_id: steward_transit_id,
                amount_in_transit: 3.0,
            },
            ..JobRuntime::default()
        });
        world.colonies[0].stock_ledger.active_round = Some(cat_sim::ledger::AccountingRound {
            worker_id: world.colonies[0].cats[0].id.clone(),
            tent_id: "accounting-restart".to_owned(),
            phase: cat_sim::ledger::AccountingPhase::Counting,
            target_stockpile_id: Some("stockpile-a".to_owned()),
            pending_stockpile_ids: vec![cat_sim::stockpiles::GENERAL_STOREHOUSE_ID.to_owned()],
            unreachable_stockpile_ids: vec!["blocked-pile".to_owned()],
            dwell_elapsed_ms: 2_000,
            topology_signature: 77,
        });

        save_world(&conn, &world).expect("save world");
        let mut loaded = load_world(&conn)
            .expect("load world")
            .expect("world should exist");

        assert_eq!(loaded.world_seed, world.world_seed);
        assert_eq!(loaded.colonies.len(), 1);
        assert_eq!(loaded.colonies[0].resources, world.colonies[0].resources);
        assert_eq!(loaded.colonies[0].resources.fibre, 6.0);
        assert_eq!(loaded.colonies[0].resources.fish, 8.75);
        assert_eq!(loaded.colonies[0].resources.hide, 4.5);
        assert_eq!(loaded.colonies[0].resources.cloth, 2.5);
        assert_eq!(loaded.colonies[0].resources.leather, 1.5);
        assert_eq!(loaded.colonies[0].cats, world.colonies[0].cats);
        assert!(loaded.colonies[0].cats[0].boosted, "boosted flag persists");
        assert_eq!(loaded.colonies[0].jobs, world.colonies[0].jobs);
        assert_eq!(loaded.colonies[0].officers, world.colonies[0].officers);
        assert_eq!(loaded.colonies[0].stockpiles, world.colonies[0].stockpiles);
        assert_eq!(loaded.colonies[0].farms, world.colonies[0].farms);
        assert_eq!(
            loaded.colonies[0].gather_spots,
            world.colonies[0].gather_spots
        );
        assert_eq!(
            loaded.colonies[0].fish_habitats,
            world.colonies[0].fish_habitats
        );
        assert_eq!(
            loaded.colonies[0].stock_ledger,
            world.colonies[0].stock_ledger
        );
        assert_eq!(
            world_tick(&mut loaded, 1_001_000),
            world_tick(&mut world, 1_001_000),
            "an active fisher follows the same post-restart tick"
        );
        assert_eq!(loaded, world, "restart does not fork fishing trajectory");
    }

    #[test]
    fn restart_mid_farm_haul_delivers_crop_exactly_once_on_later_whole_ticks() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");
        let mut world = new_world(42);
        world
            .colonies
            .push(found_colony(42, "colony-1", 1_000_000, 42));
        let colony = &mut world.colonies[0];
        colony.last_tick = 1_000_000;
        let carrier_id = colony.cats[0].id.clone();
        let gather_id = "farm-gather:farm-restart".to_owned();
        colony.stockpiles.push(cat_sim::stockpiles::Stockpile {
            id: gather_id.clone(),
            rect: ZoneRect {
                x1: colony.anchor.x,
                y1: colony.anchor.y,
                x2: colony.anchor.x,
                y2: colony.anchor.y,
            },
            accepts: [cat_sim::stockpiles::ResourceKind::Grain]
                .into_iter()
                .collect(),
            contents: Resources::default(),
        });
        colony.gather_spots.push(cat_sim::stockpiles::GatherSpot {
            stockpile_id: gather_id.clone(),
            kind: cat_sim::stockpiles::ResourceKind::Grain,
            expires_at_ms: i64::MAX,
            purpose: cat_sim::stockpiles::GatherSpotPurpose::General,
        });
        colony.farms.push(FarmPlot {
            id: "farm-restart".to_owned(),
            rect: ZoneRect {
                x1: colony.anchor.x + 2,
                y1: colony.anchor.y,
                x2: colony.anchor.x + 2,
                y2: colony.anchor.y,
            },
            crop: cat_sim::farming::CropKind::Grain,
            planted_at: 900_000,
            stage: cat_sim::farming::FarmStage::Growing,
            growth_hours: 6.0,
            fertility: 1.0,
            worker_id: None,
            work_phase: cat_sim::farming::FarmWorkPhase::Hauling,
            pending_output: 1.0,
        });
        let carrier = colony
            .cats
            .iter_mut()
            .find(|cat| cat.id == carrier_id)
            .unwrap();
        carrier.position = Position {
            map: cat_sim::entities::MapType::World,
            x: f64::from(colony.anchor.x),
            y: f64::from(colony.anchor.y),
        };
        carrier.activity = CatActivity::Returning;
        carrier.current_task = Some(TaskType::Farm);
        carrier.destination = None;
        carrier.carrying = Some(Carrying {
            kind: CarryingKind::Grain,
            amount: 2.0,
            job_ended_at: 999_000,
            source_gather_spot: Some(format!("farm-out|farm-restart|{gather_id}")),
        });
        let grain_before = colony.resources.grain;

        save_world(&conn, &world).expect("save world");
        let mut loaded = load_world(&conn)
            .expect("load world")
            .expect("world should exist");
        let _ = world_tick(&mut loaded, 1_001_000);

        let colony = &loaded.colonies[0];
        assert_eq!(colony.resources.grain, grain_before + 2.0);
        assert_eq!(
            colony
                .stockpiles
                .iter()
                .find(|pile| pile.id == gather_id)
                .unwrap()
                .contents
                .grain,
            2.0
        );
        assert!(
            colony
                .cats
                .iter()
                .find(|cat| cat.id == carrier_id)
                .unwrap()
                .carrying
                .is_none()
        );

        let _ = world_tick(&mut loaded, 1_002_000);
        assert_eq!(
            loaded.colonies[0].resources.grain,
            grain_before + 2.0,
            "a restarted basket credits exactly once"
        );
    }

    #[test]
    fn linked_expansion_source_build_job_id_round_trips_through_sqlite() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");
        let mut world = new_world(42);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000, 42));
        let builder = world.colonies[0].cats[0].id.clone();
        world.colonies[0]
            .agricultural_tiles
            .insert(TilePos { x: 30, y: 31 });
        world.colonies[0].jobs = vec![
            JobRuntime {
                id: "job-build-blocked".to_owned(),
                kind: JobKind::BuildHouse,
                status: JobStatus::Queued,
                assigned_cat: None,
                metadata: JobMetadata::Construction {
                    phase: ConstructionPhase::ConstructHouse,
                    building_type: BuildingType::Den,
                    building_id: None,
                    site: None,
                },
                ..JobRuntime::default()
            },
            JobRuntime {
                id: "job-expand-for-build".to_owned(),
                kind: JobKind::ExpandVillage,
                status: JobStatus::Active,
                assigned_cat: Some(builder),
                metadata: JobMetadata::Expansion {
                    target: TilePos { x: 13, y: 7 },
                    accepted: true,
                    source_build_job_id: Some("job-build-blocked".to_owned()),
                    wall_work_ms: 321_000,
                },
                ..JobRuntime::default()
            },
        ];

        save_world(&conn, &world).expect("save linked jobs");
        let loaded = load_world(&conn)
            .expect("load linked jobs")
            .expect("world exists");
        assert_eq!(loaded.colonies[0].jobs, world.colonies[0].jobs);
        assert_eq!(
            loaded.colonies[0].agricultural_tiles,
            world.colonies[0].agricultural_tiles
        );
        assert!(matches!(
            &loaded.colonies[0].jobs[1].metadata,
            JobMetadata::Expansion {
                source_build_job_id: Some(source),
                ..
            } if source == "job-build-blocked"
        ));
    }

    #[test]
    fn legacy_expansion_metadata_without_source_build_job_id_loads_none() {
        let metadata = parse_job_metadata(Some(
            r#"{"kind":"expansion","target":{"x":13,"y":7},"accepted":false}"#.to_owned(),
        ))
        .expect("legacy expansion metadata parses");
        assert_eq!(
            metadata,
            JobMetadata::Expansion {
                target: TilePos { x: 13, y: 7 },
                accepted: false,
                source_build_job_id: None,
                wall_work_ms: 0,
            }
        );
    }

    #[test]
    fn legacy_colony_rows_without_officers_or_stockpiles_load_empty() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");

        let mut world = new_world(20_240_703);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000_000, 42));
        save_world(&conn, &world).expect("save world");

        // Simulate a pre-P12/P16/multi-village row: additive JSON columns are NULL.
        conn.execute(
            "UPDATE colonies SET officers = NULL, stockpiles = NULL, gatherSpots = NULL,
                stockLedger = NULL, knownVillageIds = NULL, villageTradeOffers = NULL",
            [],
        )
        .expect("null columns");
        let loaded = load_world(&conn)
            .expect("load world")
            .expect("world should exist");
        assert!(loaded.colonies[0].officers.is_empty());
        assert!(loaded.colonies[0].stockpiles.is_empty());
        assert!(loaded.colonies[0].gather_spots.is_empty());
        assert!(loaded.colonies[0].known_village_ids.is_empty());
        assert!(loaded.colonies[0].village_trade_offers.is_empty());
        // A NULL ledger loads as the default (empty reported totals, never counted).
        assert_eq!(
            loaded.colonies[0].stock_ledger,
            cat_sim::ledger::StockLedger::default()
        );
    }

    #[test]
    fn aggregate_only_stock_ledger_json_migrates_without_fabricating_a_round() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");
        let mut world = new_world(42);
        world
            .colonies
            .push(found_colony(42, "colony-1", 1_000_000, 7));
        let legacy = StockLedger::counted(&world.colonies[0].resources, 999_000);
        world.colonies[0].stock_ledger = legacy.clone();
        save_world(&conn, &world).expect("save aggregate-only ledger");

        let loaded = load_world(&conn).expect("load").expect("world");
        assert_eq!(loaded.colonies[0].stock_ledger.reported, legacy.reported);
        assert_eq!(loaded.colonies[0].stock_ledger.last_counted, 999_000);
        assert!(loaded.colonies[0].stock_ledger.pile_reports.is_empty());
        assert!(loaded.colonies[0].stock_ledger.active_round.is_none());
        assert!(
            loaded.colonies[0]
                .stock_ledger
                .steward_managed_piles
                .is_empty(),
            "legacy aggregate ledgers do not fabricate Steward ownership"
        );
    }

    #[test]
    fn a_second_villages_anchor_round_trips_and_stays_distinct() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");

        let mut world = new_world(20_240_703);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000_000, 42));
        // A second village founded at a deliberately distinct anchor.
        let beta_anchor = TilePos { x: 60, y: 66 };
        let mut beta = found_colony_at(world.world_seed, "beta", 1_000_000, 4321, beta_anchor);
        beta.kind = VillageKind::Personal;
        beta.owner_player_id = Some("player-beta".to_owned());
        beta.known_village_ids.insert("colony-1".to_owned());
        world.colonies[0]
            .known_village_ids
            .insert("beta".to_owned());
        world.colonies[0].village_trade_offers.insert(
            "trade-one".to_owned(),
            cat_sim::world_tick::VillageTradeOffer {
                id: "trade-one".to_owned(),
                from_colony_id: "colony-1".to_owned(),
                to_colony_id: "beta".to_owned(),
                offered_kind: cat_sim::stockpiles::ResourceKind::Food,
                offered_amount: 4.0,
                requested_kind: cat_sim::stockpiles::ResourceKind::Materials,
                requested_amount: 2.0,
                created_at: 1_100_000,
            },
        );
        world.colonies.push(beta);

        save_world(&conn, &world).expect("save world");
        let loaded = load_world(&conn)
            .expect("load world")
            .expect("world should exist");

        let zero = loaded.colonies.iter().find(|c| c.id == "colony-1").unwrap();
        let beta = loaded.colonies.iter().find(|c| c.id == "beta").unwrap();
        assert_eq!(zero.anchor, TilePos { x: 6, y: 6 });
        assert_eq!(beta.anchor, beta_anchor);
        assert_ne!(zero.anchor, beta.anchor);
        assert_eq!(zero.kind, VillageKind::Global);
        assert_eq!(zero.owner_player_id, None);
        assert_eq!(beta.kind, VillageKind::Personal);
        assert_eq!(beta.owner_player_id.as_deref(), Some("player-beta"));
        assert_eq!(zero.known_village_ids, world.colonies[0].known_village_ids);
        assert_eq!(beta.known_village_ids, world.colonies[1].known_village_ids);
        assert_eq!(
            zero.village_trade_offers,
            world.colonies[0].village_trade_offers
        );
    }

    #[test]
    fn simultaneous_villages_round_trip_colony_local_runtime_ids() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");
        let mut world = new_world(20_240_703);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000_000, 42));
        let mut personal = found_colony_at(
            world.world_seed,
            "personal",
            1_000_000,
            4_321,
            TilePos { x: 102, y: 6 },
        );
        personal.kind = VillageKind::Personal;
        personal.owner_player_id = Some("player-personal".to_owned());
        world.colonies.push(personal);

        let _ = world_tick(&mut world, 1_001_000);
        let global_job_ids = world.colonies[0]
            .jobs
            .iter()
            .map(|job| job.id.as_str())
            .collect::<BTreeSet<_>>();
        assert!(
            world.colonies[1]
                .jobs
                .iter()
                .any(|job| global_job_ids.contains(job.id.as_str())),
            "simultaneous colony-local queues intentionally reuse runtime ids"
        );
        save_world(&conn, &world).expect("colony-scoped ids must not collide in SQLite");
        let loaded = load_world(&conn).expect("load world").expect("world");
        for expected_colony in &world.colonies {
            let loaded_colony = loaded
                .colonies
                .iter()
                .find(|colony| colony.id == expected_colony.id)
                .expect("saved colony");
            assert_eq!(
                loaded_colony
                    .cats
                    .iter()
                    .map(|entry| entry.id.as_str())
                    .collect::<Vec<_>>(),
                expected_colony
                    .cats
                    .iter()
                    .map(|entry| entry.id.as_str())
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                loaded_colony
                    .jobs
                    .iter()
                    .map(|entry| entry.id.as_str())
                    .collect::<Vec<_>>(),
                expected_colony
                    .jobs
                    .iter()
                    .map(|entry| entry.id.as_str())
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                loaded_colony
                    .buildings
                    .iter()
                    .map(|entry| entry.id.as_str())
                    .collect::<Vec<_>>(),
                expected_colony
                    .buildings
                    .iter()
                    .map(|entry| entry.id.as_str())
                    .collect::<Vec<_>>()
            );
            assert_eq!(loaded_colony.events, expected_colony.events);
            assert_eq!(loaded_colony.zones, expected_colony.zones);
            assert_eq!(loaded_colony.elections, expected_colony.elections);
            assert_eq!(loaded_colony.votes, expected_colony.votes);
            assert_eq!(loaded_colony.raiders, expected_colony.raiders);
        }
    }

    #[test]
    fn legacy_global_building_primary_key_accepts_two_colony_local_blueprints() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        conn.execute_batch(
            "CREATE TABLE buildings (
                id TEXT PRIMARY KEY,
                colonyId TEXT NOT NULL,
                type TEXT NOT NULL,
                level INTEGER NOT NULL,
                position TEXT NOT NULL,
                constructionProgress REAL NOT NULL,
                productionProgress REAL NOT NULL DEFAULT 0,
                isComplete INTEGER NOT NULL DEFAULT 0,
                assignedCatId TEXT
            );",
        )
        .expect("install shipped global-key buildings table");
        init_schema(&conn).expect("migrate remaining schema");

        let mut world = new_world(20_240_703);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000_000, 42));
        let mut personal = found_colony_at(
            world.world_seed,
            "personal",
            1_000_000,
            4_321,
            TilePos { x: 102, y: 6 },
        );
        personal.kind = VillageKind::Personal;
        personal.owner_player_id = Some("player-personal".to_owned());
        assert_eq!(
            world.colonies[0].buildings[0].id, personal.buildings[0].id,
            "founding building ids are intentionally colony-local"
        );
        world.colonies.push(personal);

        save_world(&conn, &world).expect("storage-scoped building ids must fit the legacy key");
        let loaded = load_world(&conn).expect("load world").expect("world");
        for expected_colony in &world.colonies {
            let loaded_colony = loaded
                .colonies
                .iter()
                .find(|colony| colony.id == expected_colony.id)
                .expect("saved colony");
            assert_eq!(
                loaded_colony
                    .buildings
                    .iter()
                    .map(|building| building.id.as_str())
                    .collect::<Vec<_>>(),
                expected_colony
                    .buildings
                    .iter()
                    .map(|building| building.id.as_str())
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn legacy_null_global_flags_backfill_only_the_canonical_colony() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");
        let mut world = new_world(20_240_703);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000_000, 42));
        let mut old_personal = found_colony_at(
            world.world_seed,
            "legacy-personal",
            1_000_000,
            43,
            TilePos { x: 60, y: 60 },
        );
        old_personal.kind = VillageKind::Personal;
        world.colonies.push(old_personal);
        save_world(&conn, &world).expect("save world");
        conn.execute(
            "UPDATE colonies SET isGlobal = NULL, ownerPlayerId = NULL",
            [],
        )
        .expect("erase new metadata");

        let loaded = load_world(&conn).expect("load").expect("world");

        let global = loaded
            .colonies
            .iter()
            .find(|colony| colony.id == "colony-1")
            .expect("canonical global");
        let legacy = loaded
            .colonies
            .iter()
            .find(|colony| colony.id == "legacy-personal")
            .expect("legacy personal");
        assert_eq!(global.kind, VillageKind::Global);
        assert_eq!(legacy.kind, VillageKind::Personal);
        assert_eq!(
            legacy.owner_player_id, None,
            "legacy personal stays quarantined"
        );
    }

    #[test]
    fn persistence_rejects_invalid_global_and_personal_ownership_invariants() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");
        let empty = new_world(7);
        assert!(save_world(&conn, &empty).is_err());

        let mut duplicate = new_world(7);
        duplicate
            .colonies
            .push(found_colony(7, "global-a", 1_000, 1));
        duplicate
            .colonies
            .push(found_colony(7, "global-b", 1_000, 2));
        assert!(save_world(&conn, &duplicate).is_err());

        let mut duplicate_owner = new_world(7);
        duplicate_owner
            .colonies
            .push(found_colony(7, "global", 1_000, 1));
        for id in ["personal-a", "personal-b"] {
            let mut personal = found_colony(7, id, 1_000, 2);
            personal.kind = VillageKind::Personal;
            personal.owner_player_id = Some("same-player".to_owned());
            duplicate_owner.colonies.push(personal);
        }
        assert!(save_world(&conn, &duplicate_owner).is_err());

        let mut owned_global = new_world(7);
        let mut global = found_colony(7, "global", 1_000, 1);
        global.owner_player_id = Some("impossible-owner".to_owned());
        owned_global.colonies.push(global);
        assert!(save_world(&conn, &owned_global).is_err());
    }

    #[test]
    fn failed_world_replacement_rolls_back_to_the_previous_complete_save() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");
        let mut baseline = new_world(7);
        baseline.colonies.push(found_colony(7, "global", 1_000, 1));
        save_world(&conn, &baseline).expect("save baseline");
        let expected = load_world(&conn).expect("load baseline").expect("world");

        conn.execute_batch(
            "CREATE TRIGGER reject_forced_failure
             BEFORE INSERT ON colonies
             WHEN NEW.id = 'forced-failure'
             BEGIN
               SELECT RAISE(ABORT, 'forced save failure');
             END;",
        )
        .expect("install failure trigger");

        let mut replacement = baseline;
        let mut personal = found_colony(7, "forced-failure", 2_000, 2);
        personal.kind = VillageKind::Personal;
        personal.owner_player_id = Some("player-two".to_owned());
        replacement.colonies.push(personal);

        assert!(save_world(&conn, &replacement).is_err());
        assert_eq!(
            load_world(&conn).expect("load after rollback"),
            Some(expected),
            "a failed replacement must preserve every row from the prior save"
        );
    }

    #[test]
    fn legacy_colony_row_without_an_anchor_column_loads_at_the_canonical_anchor() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");

        let mut world = new_world(20_240_703);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000_000, 42));
        save_world(&conn, &world).expect("save world");

        // Simulate a pre-multi-village row: the anchor columns are NULL.
        conn.execute("UPDATE colonies SET anchorX = NULL, anchorY = NULL", [])
            .expect("null anchor columns");
        let loaded = load_world(&conn)
            .expect("load world")
            .expect("world should exist");
        assert_eq!(loaded.colonies[0].anchor, TilePos { x: 6, y: 6 });
    }

    /// Comprehensive persistence-audit guardrail: every `ColonyRuntime`/`Cat` field is
    /// set to a distinctive, non-default value, saved, and loaded back. This is the
    /// test that would have caught `items`/craft-progress silently dropping on
    /// restart — any future field added to either struct without wiring it into
    /// `save_colony`/`load_colony`/`save_cat`/`load_cats` should make this test fail
    /// (either by comparison against a stale expectation, or because the loaded value
    /// stays at its `Default` while the saved value does not).
    ///
    /// `trader`, `last_trader_departed_at`, and the end-of-tick-only
    /// `pending_scout_delivery_tiles` are intentionally excluded from the round trip
    /// (see the doc comments on `ColonyRuntime` and in `load_colony`). A restart is
    /// documented to drop an in-flight trader visit. Scout notebooks persist because
    /// dropping them can make a returning scout deliver no knowledge after a restart.
    /// This test asserts that documented contract explicitly so a future change shows
    /// up here too.
    #[test]
    fn save_world_load_world_round_trips_every_field_in_the_persistence_audit() {
        use cat_sim::{
            entities::{CarryingKind, MapType},
            items::{Item, ItemKind, Material},
            ledger::StockLedger,
            officers::OfficerRole,
            stockpiles::{GatherSpot, ResourceKind, Stockpile},
            world_tick::{
                ElectionKind, ElectionRuntime, RaiderRuntime, UpgradeLevels, VoteRuntime,
            },
            zones::{ZoneKind, ZoneRect},
        };

        let conn = Connection::open_in_memory().expect("open sqlite");
        init_schema(&conn).expect("init schema");

        let mut world = new_world(20_260_711);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-audit", 5_000_000, 7));
        let colony = &mut world.colonies[0];

        // --- top-level scalars ---------------------------------------------------
        colony.leader_id = Some(colony.cats[0].id.clone());
        colony.status = ColonyStatus::Thriving;
        colony.resources = Resources {
            food: 11.0,
            fish: 11.5,
            water: 12.0,
            herbs: 13.0,
            catnip: 13.1,
            grain: 13.2,
            flour: 13.3,
            materials: 14.0,
            refined: 15.0,
            weapons: 16.0,
            armor: 17.0,
            planks: 18.0,
            logs: 18.1,
            lumber: 18.2,
            blocks: 19.0,
            tools: 20.0,
            fibre: 21.0,
            hide: 22.0,
            cloth: 23.0,
            leather: 24.0,
            ore: 25.5,
            metal: 26.5,
            blessings: 25.0,
        };
        colony.automation_tier = 3.5;
        colony.global_upgrade_points = 42.0;
        colony.ritual_requested_at = Some(5_100_000);
        colony.critical_since = Some(5_200_000);
        colony.threat_pressure = 63.5;
        colony.last_raid_at = Some(5_300_000);
        colony.active_raid = Some("raid-1".to_owned());
        colony.raid_clicks = 4.0;
        colony.coin = 88.5;
        colony.wood_craft_progress = 111.0;
        colony.stone_craft_progress = 222.0;
        colony.clothier_craft_progress = 333.0;
        colony.tannery_craft_progress = 444.0;
        colony.metal_forge_progress = 555.0;
        colony.run_number = 3;
        colony.run_started_at = 4_900_000;
        colony.created_at = 4_800_000;
        colony.last_player_activity_at = Some(5_400_000);
        colony.last_leader_research_choice_at = Some(5_350_000);
        colony.last_tithe_at = Some(5_360_000);
        colony.last_offering_at = Some(5_370_000);
        colony.last_tick = 5_500_000;
        colony.test_time_scale = 2.5;
        colony.test_resource_decay_multiplier = 1.75;
        colony.test_resilience_hours_override = Some(9.5);
        colony.test_critical_ms_override = 123_456;
        colony.test_rng_seed = Some(99);

        // --- upgrade tree / levels -------------------------------------------------
        colony.upgrade_levels = UpgradeLevels {
            click_power: 1,
            supply_speed: 2,
            hunt_mastery: 3,
            build_mastery: 4,
            ritual_mastery: 5,
            resilience: 6,
        };
        colony.upgrade_tree.owned_node_ids = vec![
            "era1-storage".to_owned(),
            "era2-workshop".to_owned(),
            "research_hut_foundations".to_owned(),
            "logistics_basics".to_owned(),
            "food_storage_stores".to_owned(),
        ];
        colony.upgrade_tree.research_points = 17.5;

        // --- fog / claimed tiles ----------------------------------------------------
        colony.claimed_tiles.push(TilePos { x: 40, y: 41 });
        colony.revealed_tiles.insert(TilePos { x: 40, y: 41 });
        colony.provisional_tiles.insert(
            colony.cats[0].id.clone(),
            [TilePos { x: 42, y: 43 }, TilePos { x: 44, y: 45 }]
                .into_iter()
                .collect(),
        );
        colony
            .pending_scout_delivery_tiles
            .insert(TilePos { x: 46, y: 47 });

        // --- officers / stockpiles / gather spots / ledger / items ------------------
        colony
            .officers
            .insert(OfficerRole::Captain, colony.cats[1].id.clone());
        colony.stockpiles.push(Stockpile {
            id: "stockpile-audit".to_owned(),
            rect: ZoneRect {
                x1: 8,
                y1: 8,
                x2: 9,
                y2: 9,
            },
            accepts: [ResourceKind::Materials].into_iter().collect(),
            contents: Resources {
                materials: 6.0,
                ..Resources::default()
            },
        });
        colony.stockpiles.push(Stockpile {
            id: "gather-audit".to_owned(),
            rect: ZoneRect {
                x1: 30,
                y1: 30,
                x2: 30,
                y2: 30,
            },
            accepts: [ResourceKind::Water].into_iter().collect(),
            contents: Resources::default(),
        });
        colony.gather_spots.push(GatherSpot {
            stockpile_id: "gather-audit".to_owned(),
            kind: ResourceKind::Water,
            expires_at_ms: 5_600_000,
            purpose: cat_sim::stockpiles::GatherSpotPurpose::General,
        });
        colony.farms.push(FarmPlot {
            id: "farm-audit".to_owned(),
            rect: ZoneRect {
                x1: 12,
                y1: 12,
                x2: 13,
                y2: 13,
            },
            crop: cat_sim::farming::CropKind::Catnip,
            planted_at: 5_250_000,
            stage: cat_sim::farming::FarmStage::Mature,
            worker_id: None,
            work_phase: cat_sim::farming::FarmWorkPhase::WaitingForWorker,
            pending_output: 0.0,
            growth_hours: 14.0,
            fertility: 1.5,
        });
        colony.stock_ledger = StockLedger::counted(&colony.resources, 5_500_000);
        colony
            .items
            .add(Item::new(ItemKind::Mug, Material::Wood, 2), 7, 1.0);
        colony
            .items
            .add(Item::new(ItemKind::Clothing, Material::Leather, 4), 1, 1.0);
        colony.items.wear(ItemKind::Mug, 2);

        // --- jobs / buildings / events / zones / elections / votes / raiders -------
        let mover_cat_id = colony.cats[2].id.clone();
        colony.jobs.push(JobRuntime {
            id: "job-audit".to_owned(),
            kind: JobKind::HaulGatherSpot,
            status: JobStatus::Active,
            assigned_cat: Some(mover_cat_id),
            metadata: JobMetadata::GatherHaul {
                stockpile_id: "gather-audit".to_owned(),
                site: Some(TilePos { x: 30, y: 30 }),
                accepted: true,
            },
            ..JobRuntime::default()
        });
        colony.buildings.push(BuildingRuntime {
            id: "building-audit".to_owned(),
            building_type: BuildingType::Smithy,
            level: 2,
            position: TilePos { x: 20, y: 21 },
            is_complete: true,
            construction_progress: 100,
            production_progress: 5.5,
            assigned_cat: Some(colony.cats[3].id.clone()),
            automated_by: Some(OfficerRole::Captain),
            production_queue: Vec::new(),
            production_paused: false,
        });
        colony.events.push(EventLog {
            id: "event-audit".to_owned(),
            at_ms: 5_450_000,
            kind: EventKind::Raid(RaidPhase::Repelled),
            message: "A raid was repelled".to_owned(),
        });
        colony.zones.push(ZoneRuntime {
            rect: ZoneRect {
                x1: 1,
                y1: 2,
                x2: 3,
                y2: 4,
            },
            kind: ZoneKind::Gather,
            created_at: 5_000_100,
            expires_at: 5_100_100,
            player_id: Some(4242),
        });
        colony.elections.push(ElectionRuntime {
            id: "election-audit".to_owned(),
            opened_at: 5_000_200,
            closes_at: 5_100_200,
            resolved_at: Some(5_100_250),
            winner_cat_id: Some(colony.cats[0].id.clone()),
            kind: ElectionKind::VoteKick,
        });
        colony.votes.push(VoteRuntime {
            id: "vote-audit".to_owned(),
            election_id: "election-audit".to_owned(),
            voter_id: "player-audit".to_owned(),
            cat_id: colony.cats[0].id.clone(),
            weight: 1.0,
        });
        colony.raiders.push(RaiderRuntime {
            id: "raider-audit".to_owned(),
            raid_id: "raid-1".to_owned(),
            position: Position {
                map: MapType::World,
                x: 5.0,
                y: 6.0,
            },
            destination: Some(Position {
                map: MapType::World,
                x: 9.0,
                y: 10.0,
            }),
            attack: 12.0,
            defense: 8.0,
            health: 30.0,
        });

        // --- cats: give two cats every optional field a distinctive value -----------
        let cat_b_id = colony.cats[1].id.clone();
        {
            let cat_a = &mut colony.cats[0];
            cat_a.parent_ids = vec![Some("ancestor-1".to_owned()), None];
            cat_a.death_time = None;
            cat_a.current_task = Some(TaskType::Hunt);
            cat_a.destination = Some(Position {
                map: MapType::World,
                x: 3.0,
                y: 4.0,
            });
            cat_a.carrying = Some(Carrying {
                kind: CarryingKind::Materials,
                amount: 6.5,
                job_ended_at: 5_500_500,
                source_gather_spot: Some("gather-audit".to_owned()),
            });
            cat_a.activity = CatActivity::Returning;
            cat_a.is_pregnant = true;
            cat_a.pregnancy_due_time = Some(5_600_500);
            cat_a.age_hours = 30.5;
            cat_a.pregnancy_due_age_hours = Some(36.0);
            cat_a.pregnancy_mate_id = Some(cat_b_id.clone());
            cat_a.sprite_params = Some(BTreeMap::from([(
                "coat".to_owned(),
                serde_json::json!("tabby"),
            )]));
            cat_a.specialization = Some(CatSpecialization::Warrior);
            cat_a.role_xp = RoleXp {
                hunter: 1.0,
                architect: 2.0,
                ritualist: 3.0,
                warrior: 4.0,
            };
            cat_a.gain_skill(Labor::Hunt, 9.0);
            cat_a.gain_skill(Labor::Craft, 3.0);
            cat_a.boosted = true;
            cat_a.preferred_labors = BTreeSet::from([Labor::Hunt, Labor::Research]);
        }
        colony.cats[1].death_time = Some(5_700_000);

        let expected = world.clone();
        save_world(&conn, &world).expect("save world");
        let loaded = load_world(&conn)
            .expect("load world")
            .expect("world should exist");

        assert_eq!(loaded.colonies.len(), expected.colonies.len());
        let loaded_colony = &loaded.colonies[0];
        let expected_colony = &expected.colonies[0];

        // Every field except the three documented-transient ones must round-trip
        // exactly. Compare via a clone with those fields reset to what
        // `load_colony` is documented to produce, so any other drift fails the
        // `assert_eq!` on the whole struct.
        let mut expected_after_reload = expected_colony.clone();
        expected_after_reload.trader = None;
        expected_after_reload.last_trader_departed_at = None;
        expected_after_reload.pending_scout_delivery_tiles.clear();

        assert_eq!(loaded_colony, &expected_after_reload);
        assert!(
            loaded_colony.decoration_cache.is_empty(),
            "runtime terrain caches must always reload cold"
        );

        // Belt-and-suspenders explicit assertions on the fields this test exists to
        // guard (readable failure messages instead of one big struct diff).
        assert_eq!(loaded_colony.items, expected_colony.items);
        assert_eq!(
            loaded_colony.wood_craft_progress,
            expected_colony.wood_craft_progress
        );
        assert_eq!(
            loaded_colony.stone_craft_progress,
            expected_colony.stone_craft_progress
        );
        assert_eq!(
            loaded_colony.clothier_craft_progress,
            expected_colony.clothier_craft_progress
        );
        assert_eq!(
            loaded_colony.tannery_craft_progress,
            expected_colony.tannery_craft_progress
        );
        assert_eq!(
            loaded_colony.metal_forge_progress,
            expected_colony.metal_forge_progress
        );
        assert_eq!(loaded_colony.coin, expected_colony.coin);
        assert_eq!(
            loaded_colony.last_leader_research_choice_at,
            expected_colony.last_leader_research_choice_at
        );
        assert_eq!(loaded_colony.last_tithe_at, expected_colony.last_tithe_at);
        assert_eq!(
            loaded_colony.last_offering_at,
            expected_colony.last_offering_at
        );
        assert_eq!(
            loaded_colony.provisional_tiles,
            expected_colony.provisional_tiles
        );

        // Documented transient fields: confirm the *reset*, not just its absence from
        // the equality check above.
        assert_eq!(loaded_colony.trader, None);
        assert_eq!(loaded_colony.last_trader_departed_at, None);
        assert!(loaded_colony.pending_scout_delivery_tiles.is_empty());
    }
}
