/**
 * World map persistence (ported from the former convex/worldMap.ts).
 *
 * Chunked procedural generation: lib/game/worldGen.ts produces tiles
 * deterministically from the colony's worldSeed; this module stores and
 * serves them. Generation is idempotent per chunk.
 */

import { and, eq, gte, lt } from "drizzle-orm";
import { nanoid } from "nanoid";

import type { GameDb } from "@/db/client";
import { colonies, type WorldTileRow, worldTiles } from "@/db/schema";
import { generateWorldChunk } from "@/lib/game/terrainWorld";
import { getColonyPosition } from "@/lib/game/worldGen";

const CHUNK_SIZE = 12;

export function getChunkTiles(
	db: GameDb,
	colonyId: string,
	chunkX: number,
	chunkY: number,
): WorldTileRow[] {
	const minX = chunkX * CHUNK_SIZE;
	const minY = chunkY * CHUNK_SIZE;

	return db
		.select()
		.from(worldTiles)
		.where(
			and(
				eq(worldTiles.colonyId, colonyId),
				gte(worldTiles.x, minX),
				lt(worldTiles.x, minX + CHUNK_SIZE),
				gte(worldTiles.y, minY),
				lt(worldTiles.y, minY + CHUNK_SIZE),
			),
		)
		.all();
}

function chunkExists(
	db: GameDb,
	colonyId: string,
	chunkX: number,
	chunkY: number,
): boolean {
	const minX = chunkX * CHUNK_SIZE;
	const minY = chunkY * CHUNK_SIZE;

	const row = db
		.select({ _id: worldTiles._id })
		.from(worldTiles)
		.where(
			and(
				eq(worldTiles.colonyId, colonyId),
				gte(worldTiles.x, minX),
				lt(worldTiles.x, minX + CHUNK_SIZE),
				gte(worldTiles.y, minY),
				lt(worldTiles.y, minY + CHUNK_SIZE),
			),
		)
		.limit(1)
		.get();

	return row !== undefined;
}

/** Generate and store a chunk's tiles if they don't exist yet. */
export function ensureChunk(
	db: GameDb,
	colonyId: string,
	chunkX: number,
	chunkY: number,
): void {
	const colony = db
		.select()
		.from(colonies)
		.where(eq(colonies._id, colonyId))
		.get();
	if (!colony) {
		throw new Error("Colony not found");
	}

	const worldSeed = colony.worldSeed ?? colony.createdAt;
	const colonyPos = getColonyPosition();

	const tiles = generateWorldChunk(
		chunkX,
		chunkY,
		worldSeed,
		colonyPos.x,
		colonyPos.y,
	);

	// One transaction per chunk: a partially-inserted chunk would read back as
	// "exists" and never finish generating, leaving a hole in the map.
	db.transaction((tx) => {
		if (chunkExists(tx as unknown as GameDb, colonyId, chunkX, chunkY)) {
			return;
		}

		for (const tile of tiles) {
			tx.insert(worldTiles)
				.values({
					_id: nanoid(),
					colonyId,
					...tile,
				})
				.onConflictDoNothing()
				.run();
		}
	});
}

/** Initialize the starting world: 3x3 chunks around the village (chunk 0,0). */
export function initializeWorldMap(db: GameDb, colonyId: string): void {
	for (let chunkY = -1; chunkY <= 1; chunkY++) {
		for (let chunkX = -1; chunkX <= 1; chunkX++) {
			ensureChunk(db, colonyId, chunkX, chunkY);
		}
	}
}
