/**
 * One-shot dashboard fetch (fallback for clients without SSE, e.g. tests).
 */

import { NextResponse } from "next/server";

import { getDb } from "@/db/client";
import { ensureGlobalState, getGlobalDashboard } from "@/server/game";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";
export const revalidate = 0;
export const fetchCache = "force-no-store";

export async function GET() {
	try {
		const db = getDb();
		ensureGlobalState(db); // transactional bootstrap
		return NextResponse.json(getGlobalDashboard(db), {
			headers: { "Cache-Control": "no-store, max-age=0" },
		});
	} catch (err) {
		console.error("[dashboard] failed:", err);
		return NextResponse.json(
			{ ok: false, message: "Game backend unavailable." },
			{ status: 503 },
		);
	}
}
