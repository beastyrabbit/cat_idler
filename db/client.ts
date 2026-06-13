/**
 * SQLite client factory.
 *
 * The web server (Next.js route handlers) and the worker are separate
 * processes sharing one database file — WAL mode + busy_timeout make
 * concurrent access safe. Tests pass ':memory:' for isolated databases.
 */

import fs from "node:fs";
import path from "node:path";

import Database from "better-sqlite3";
import { drizzle } from "drizzle-orm/better-sqlite3";
import { migrate } from "drizzle-orm/better-sqlite3/migrator";

import * as schema from "./schema";

export type GameDb = ReturnType<typeof createDb>;

const MIGRATIONS_FOLDER = path.join(process.cwd(), "db", "migrations");

export function createDb(
	dbPath: string = process.env.GAME_DB_PATH ?? "data/game.db",
): ReturnType<typeof drizzle<typeof schema>> {
	if (dbPath !== ":memory:") {
		fs.mkdirSync(path.dirname(path.resolve(dbPath)), { recursive: true });
	}

	const sqlite = new Database(dbPath);
	sqlite.pragma("journal_mode = WAL");
	sqlite.pragma("busy_timeout = 5000");
	sqlite.pragma("synchronous = NORMAL");

	const db = drizzle(sqlite, { schema });
	migrate(db, { migrationsFolder: MIGRATIONS_FOLDER });
	return db;
}

let singleton: GameDb | null = null;

/** Process-wide shared handle (web server or worker). */
export function getDb(): GameDb {
	if (!singleton) {
		singleton = createDb();
	}
	return singleton;
}
