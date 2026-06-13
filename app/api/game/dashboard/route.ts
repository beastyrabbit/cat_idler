/**
 * One-shot dashboard fetch (fallback for clients without SSE, e.g. tests).
 */

import { NextResponse } from "next/server";

import { getDb } from "@/db/client";
import { ensureGlobalColony, getGlobalDashboard } from "@/server/game";

export const dynamic = "force-dynamic";

export async function GET() {
	const db = getDb();
	ensureGlobalColony(db);
	return NextResponse.json(getGlobalDashboard(db));
}
