import { type NextRequest, NextResponse } from "next/server";

import { subscriberHashFromHeaders } from "@/server/voterIdentity";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

export async function GET(req: NextRequest) {
	try {
		// SHA-256 hash of IP + salt, truncated to 16 hex chars.
		return NextResponse.json({ hash: subscriberHashFromHeaders(req.headers) });
	} catch (err) {
		console.error("subscriber-hash: failed to generate hash", err);
		return NextResponse.json({ hash: "unknown" }, { status: 500 });
	}
}
