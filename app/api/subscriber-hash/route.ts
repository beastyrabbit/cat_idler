import { NextRequest, NextResponse } from "next/server";

export async function GET(req: NextRequest) {
  const forwarded = req.headers.get("x-forwarded-for");
  const ip = forwarded?.split(",")[0]?.trim() ?? "unknown";

  // Simple hash — not cryptographic, just a stable anonymous ID
  const encoder = new TextEncoder();
  const data = encoder.encode(ip + "catford-examiner-salt-2026");
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hash = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");

  return NextResponse.json({ hash: hash.slice(0, 16) });
}
