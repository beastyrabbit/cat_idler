import { NextRequest, NextResponse } from "next/server";

export async function GET(req: NextRequest) {
  try {
    const forwarded = req.headers.get("x-forwarded-for");
    const ip = forwarded?.split(",")[0]?.trim() ?? "unknown";

    // SHA-256 hash of IP + salt, truncated to 16 hex chars.
    // Not used for security — just a stable anonymous subscriber ID.
    const encoder = new TextEncoder();
    const data = encoder.encode(ip + "catford-examiner-salt-2026");
    const hashBuffer = await crypto.subtle.digest("SHA-256", data);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    const hash = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");

    return NextResponse.json({ hash: hash.slice(0, 16) });
  } catch (err) {
    console.error("subscriber-hash: failed to generate hash", err);
    return NextResponse.json({ hash: "unknown" }, { status: 500 });
  }
}
