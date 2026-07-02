/**
 * World chunk tiles for the map UI.
 *
 * GET /api/game/chunks?x=<chunkX>&y=<chunkY> — returns the stored tiles of one
 * 12x12 chunk for the global colony. The world is effectively infinite: a
 * requested chunk that has never been generated is generated on demand (deter-
 * ministically from the colony's worldSeed) as long as it falls inside the
 * renderable window. Chunks outside the window are never generated and return
 * an empty list, so panning can't drive unbounded generation.
 */

import { NextResponse } from "next/server";

import { getDb } from "@/db/client";
import { chunkWindow, DEFAULT_ISO_GEOMETRY } from "@/lib/game/isoProjection";
import { ensureGlobalState } from "@/server/game";
import { ensureChunk, getChunkTiles } from "@/server/worldMap";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const WINDOW = chunkWindow(DEFAULT_ISO_GEOMETRY);

function inWindow(chunkX: number, chunkY: number): boolean {
	return (
		chunkX >= WINDOW.min &&
		chunkX <= WINDOW.max &&
		chunkY >= WINDOW.min &&
		chunkY <= WINDOW.max
	);
}

export async function GET(request: Request) {
	const url = new URL(request.url);
	const chunkX = Number(url.searchParams.get("x"));
	const chunkY = Number(url.searchParams.get("y"));

	if (!Number.isInteger(chunkX) || !Number.isInteger(chunkY)) {
		return NextResponse.json(
			{ ok: false, message: "Invalid chunk coordinates." },
			{ status: 400 },
		);
	}

	try {
		const db = getDb();
		const colonyId = ensureGlobalState(db);
		// Generate the chunk on first visit — but only inside the renderable
		// window, so a client can't request faraway chunks to bloat the DB.
		if (inWindow(chunkX, chunkY)) {
			ensureChunk(db, colonyId, chunkX, chunkY);
		}
		const tiles = getChunkTiles(db, colonyId, chunkX, chunkY);
		return NextResponse.json(
			{ tiles },
			{
				headers: {
					// Tiles are immutable in this phase; let the browser cache them
					// briefly to avoid refetch storms while panning.
					"Cache-Control": "private, max-age=30",
				},
			},
		);
	} catch (err) {
		console.error("[chunks] failed:", err);
		return NextResponse.json(
			{ ok: false, message: "Game backend unavailable." },
			{ status: 503 },
		);
	}
}
