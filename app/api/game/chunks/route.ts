/**
 * World chunk tiles for the map UI.
 *
 * GET /api/game/chunks?x=<chunkX>&y=<chunkY> — returns the stored tiles
 * of one 12x12 chunk for the global colony. Ungenerated chunks return an
 * empty list (rendered as uncharted territory); generation only happens
 * through the simulation, never from map panning.
 */

import { NextResponse } from "next/server";

import { getDb } from "@/db/client";
import { ensureGlobalState } from "@/server/game";
import { getChunkTiles } from "@/server/worldMap";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

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
