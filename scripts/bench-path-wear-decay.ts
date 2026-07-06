import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { performance } from "node:perf_hooks";

import Database from "better-sqlite3";

const CHUNK_TILES = 12 * 12;
const COLONY_ID = "bench-colony";
const DECAY_AMOUNT = 1;
const ITERATIONS = 80;
const WARMUP = 10;

const CASES = [
	{ label: "fresh", chunks: 9 },
	{ label: "~200 chunks", chunks: 200 },
	{ label: "~1000 chunks", chunks: 1000 },
] as const;

type TilePattern = {
	id: string;
	pathWear: number;
	overlayFeature: string | null;
};

function percentile(values: number[], p: number): number {
	const sorted = [...values].sort((a, b) => a - b);
	const index = Math.min(
		sorted.length - 1,
		Math.floor((sorted.length - 1) * p),
	);
	return sorted[index];
}

function createBenchDb(index: "legacy" | "partial"): Database.Database {
	const dir = fs.mkdtempSync(path.join(os.tmpdir(), "path-wear-decay-"));
	const dbPath = path.join(dir, "bench.db");
	const db = new Database(dbPath);
	db.pragma("journal_mode = WAL");
	db.pragma("busy_timeout = 5000");
	db.pragma("synchronous = NORMAL");
	db.exec(`
		CREATE TABLE worldTiles (
			id TEXT PRIMARY KEY NOT NULL,
			colonyId TEXT NOT NULL,
			x INTEGER NOT NULL,
			y INTEGER NOT NULL,
			type TEXT NOT NULL,
			resources TEXT NOT NULL,
			maxResources TEXT NOT NULL,
			dangerLevel REAL NOT NULL,
			pathWear REAL NOT NULL,
			lastDepleted INTEGER NOT NULL,
			overlayFeature TEXT
		);
		CREATE INDEX worldTiles_by_colony ON worldTiles (colonyId);
		CREATE UNIQUE INDEX worldTiles_by_colony_position ON worldTiles (colonyId, x, y);
	`);
	if (index === "partial") {
		db.exec(
			"CREATE INDEX worldTiles_by_colony_path_wear_nonzero ON worldTiles (colonyId, pathWear) WHERE pathWear > 0;",
		);
	}
	return db;
}

function tilePatterns(tileCount: number): TilePattern[] {
	const mutableWorn = Math.min(384, Math.max(24, Math.floor(tileCount * 0.01)));
	const patterns: TilePattern[] = [];
	for (let i = 0; i < mutableWorn; i++) {
		const mod = i % 6;
		patterns.push({
			id: `tile-${i}`,
			pathWear:
				mod === 0
					? 80
					: mod === 1
						? 70
						: mod === 2
							? 45
							: mod === 3
								? 63
								: mod === 4
									? 1
									: 0.5,
			overlayFeature: mod === 5 ? "road_built" : null,
		});
	}
	return patterns;
}

function seedTiles(db: Database.Database, chunks: number): TilePattern[] {
	const tileCount = chunks * CHUNK_TILES;
	const patterns = tilePatterns(tileCount);
	const byId = new Map(patterns.map((pattern) => [pattern.id, pattern]));
	const insert = db.prepare(`
		INSERT INTO worldTiles (
			id, colonyId, x, y, type, resources, maxResources, dangerLevel,
			pathWear, lastDepleted, overlayFeature
		) VALUES (?, ?, ?, ?, 'field', '{"food":0,"herbs":0,"water":0}', '{"food":0,"herbs":0}', 0, ?, 0, ?)
	`);
	const insertAll = db.transaction(() => {
		for (let i = 0; i < tileCount; i++) {
			const id = `tile-${i}`;
			const pattern = byId.get(id);
			insert.run(
				id,
				COLONY_ID,
				i % 1200,
				Math.floor(i / 1200),
				pattern?.pathWear ?? 0,
				pattern?.overlayFeature ?? null,
			);
		}
	});
	insertAll();
	return patterns;
}

function resetPatterns(db: Database.Database, patterns: TilePattern[]): void {
	const reset = db.prepare(
		"UPDATE worldTiles SET pathWear = ?, overlayFeature = ? WHERE id = ?",
	);
	const resetAll = db.transaction(() => {
		for (const pattern of patterns) {
			reset.run(pattern.pathWear, pattern.overlayFeature, pattern.id);
		}
	});
	resetAll();
}

function measure(
	label: string,
	runOnce: () => void,
	reset: () => void,
): string {
	const samples: number[] = [];
	for (let i = 0; i < WARMUP + ITERATIONS; i++) {
		reset();
		const start = performance.now();
		runOnce();
		const elapsed = performance.now() - start;
		if (i >= WARMUP) {
			samples.push(elapsed);
		}
	}
	return `${label} p50=${percentile(samples, 0.5).toFixed(3)}ms p95=${percentile(samples, 0.95).toFixed(3)}ms`;
}

function runLegacyLoop(db: Database.Database): void {
	const wornTiles = db
		.prepare(
			"SELECT id, pathWear, overlayFeature FROM worldTiles WHERE colonyId = ? AND pathWear > 0",
		)
		.all(COLONY_ID) as Array<{
		id: string;
		pathWear: number;
		overlayFeature: string | null;
	}>;
	const update = db.prepare("UPDATE worldTiles SET pathWear = ? WHERE id = ?");
	const tx = db.transaction(() => {
		for (const worn of wornTiles) {
			if (worn.overlayFeature === "road_built") {
				continue;
			}
			let next = worn.pathWear;
			if (worn.pathWear >= 70) {
				next = Math.max(63, worn.pathWear - DECAY_AMOUNT);
			} else if (worn.pathWear > 62) {
				continue;
			} else {
				next = Math.max(1, worn.pathWear - DECAY_AMOUNT);
			}
			if (next !== worn.pathWear) {
				update.run(next, worn.id);
			}
		}
	});
	tx();
}

function runSqlUpdate(db: Database.Database): void {
	db.prepare(
		`
		UPDATE worldTiles
		SET pathWear = CASE
			WHEN pathWear >= 70 THEN max(63, pathWear - @decayAmount)
			ELSE max(1, pathWear - @decayAmount)
		END
		WHERE colonyId = @colonyId
			AND pathWear > 0
			AND (overlayFeature IS NULL OR overlayFeature <> 'road_built')
			AND (pathWear >= 70 OR pathWear <= 62)
			AND pathWear <> 1
	`,
	).run({ colonyId: COLONY_ID, decayAmount: DECAY_AMOUNT });
}

for (const benchCase of CASES) {
	const legacy = createBenchDb("legacy");
	const legacyPatterns = seedTiles(legacy, benchCase.chunks);
	const optimized = createBenchDb("partial");
	const optimizedPatterns = seedTiles(optimized, benchCase.chunks);

	console.log(`${benchCase.label} (${benchCase.chunks * CHUNK_TILES} tiles)`);
	console.log(
		measure(
			"  legacy loop",
			() => runLegacyLoop(legacy),
			() => resetPatterns(legacy, legacyPatterns),
		),
	);
	console.log(
		measure(
			"  sql update ",
			() => runSqlUpdate(optimized),
			() => resetPatterns(optimized, optimizedPatterns),
		),
	);
}
