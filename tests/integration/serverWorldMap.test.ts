/**
 * Integration tests for the shrine + world bootstrap (Phase 1: map-first UI).
 */

import { eq } from "drizzle-orm";
import { beforeEach, describe, expect, it } from "vitest";

import { createDb, type GameDb } from "@/db/client";
import { buildings, colonies, worldTiles } from "@/db/schema";
import { VILLAGE_ANCHOR } from "@/lib/game/villageLayout";
import { ensureGlobalColony, getGlobalDashboard } from "@/server/game";
import { ensureChunk, getChunkTiles } from "@/server/worldMap";

let db: GameDb;

beforeEach(() => {
	db = createDb(":memory:");
});

describe("shrine bootstrap", () => {
	it("creates a level-1 shrine at the village center", () => {
		const colony = ensureGlobalColony(db);

		const shrines = db
			.select()
			.from(buildings)
			.where(eq(buildings.colonyId, colony._id))
			.all()
			.filter((b) => b.type === "shrine");

		expect(shrines).toHaveLength(1);
		expect(shrines[0].level).toBe(1);
		expect(shrines[0].position).toEqual({ x: 0, y: 0 });
		expect(shrines[0].constructionProgress).toBe(100);
	});

	it("seeds the starting 3x3 world chunks", () => {
		const colony = ensureGlobalColony(db);

		const tiles = db
			.select({ _id: worldTiles._id })
			.from(worldTiles)
			.where(eq(worldTiles.colonyId, colony._id))
			.all();

		// 3x3 chunks of 12x12 tiles
		expect(tiles).toHaveLength(9 * 12 * 12);
	});

	it("is idempotent across repeated bootstraps", () => {
		ensureGlobalColony(db);
		const colony = ensureGlobalColony(db);

		const shrines = db
			.select()
			.from(buildings)
			.where(eq(buildings.colonyId, colony._id))
			.all()
			.filter((b) => b.type === "shrine");
		expect(shrines).toHaveLength(1);

		const tileCount = db
			.select({ _id: worldTiles._id })
			.from(worldTiles)
			.where(eq(worldTiles.colonyId, colony._id))
			.all().length;
		expect(tileCount).toBe(9 * 12 * 12);
	});

	it("generates tiles deterministically from the world seed", () => {
		const colony = ensureGlobalColony(db);
		const first = getChunkTiles(db, colony._id, 0, 0)
			.sort((a, b) => a.y - b.y || a.x - b.x)
			.map((t) => t.type);
		expect(first).toHaveLength(144);

		// Re-generating the same chunk in a second DB with the same seed
		// yields identical tile types.
		const db2 = createDb(":memory:");
		const colony2 = ensureGlobalColony(db2);
		db2.delete(worldTiles).where(eq(worldTiles.colonyId, colony2._id)).run();
		db2
			.update(colonies)
			.set({ worldSeed: colony.worldSeed })
			.where(eq(colonies._id, colony2._id))
			.run();
		ensureChunk(db2, colony2._id, 0, 0);

		const second = getChunkTiles(db2, colony2._id, 0, 0)
			.sort((a, b) => a.y - b.y || a.x - b.x)
			.map((t) => t.type);
		expect(second).toEqual(first);
	});
});

describe("getChunkTiles", () => {
	it("returns 144 tiles for a generated chunk", () => {
		const colony = ensureGlobalColony(db);
		const tiles = getChunkTiles(db, colony._id, 0, 0);
		expect(tiles).toHaveLength(144);
		for (const tile of tiles) {
			expect(tile.x).toBeGreaterThanOrEqual(0);
			expect(tile.x).toBeLessThan(12);
			expect(tile.y).toBeGreaterThanOrEqual(0);
			expect(tile.y).toBeLessThan(12);
		}
	});

	it("returns an empty list for ungenerated chunks", () => {
		const colony = ensureGlobalColony(db);
		expect(getChunkTiles(db, colony._id, 50, 50)).toEqual([]);
	});

	it("ensureChunk generates missing chunks exactly once", () => {
		const colony = ensureGlobalColony(db);

		ensureChunk(db, colony._id, 2, 2);
		ensureChunk(db, colony._id, 2, 2);

		expect(getChunkTiles(db, colony._id, 2, 2)).toHaveLength(144);
	});
});

describe("dashboard map payload", () => {
	it("exposes buildings with world positions and the village anchor", () => {
		ensureGlobalColony(db);
		const dashboard = getGlobalDashboard(db);

		expect(dashboard?.anchor).toEqual(VILLAGE_ANCHOR);

		const shrine = dashboard?.buildings.find((b) => b.type === "shrine");
		expect(shrine).toBeDefined();
		// shrine local (0,0) maps onto the world anchor
		expect(shrine?.worldPosition).toEqual(VILLAGE_ANCHOR);
	});
});
