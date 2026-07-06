/**
 * Integration tests for the actions route's identity + abuse guards.
 *
 * Exercises the real POST handler against a temp SQLite DB: signed-session
 * minting (presence), signature verification/rejection, legacy-session upgrade,
 * the per-session rate limit, and payload size/shape guards.
 */

import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import { afterAll, beforeAll, describe, expect, it } from "vitest";

import { signSession, verifySession } from "@/lib/game/identity";

const SECRET = "route-test-secret";
const DB_PATH = path.join(
	os.tmpdir(),
	`cat-idler-actions-${Math.random().toString(36).slice(2)}.db`,
);

// Import the route only after the environment is configured — getDb() reads
// GAME_DB_PATH lazily and getSessionSecret() reads SESSION_HMAC_SECRET per call.
let POST: (request: Request) => Promise<Response>;

beforeAll(async () => {
	process.env.GAME_DB_PATH = DB_PATH;
	process.env.SESSION_HMAC_SECRET = SECRET;
	POST = (await import("@/app/api/game/actions/route")).POST;
});

afterAll(() => {
	for (const suffix of ["", "-wal", "-shm"]) {
		try {
			fs.unlinkSync(`${DB_PATH}${suffix}`);
		} catch {
			// best-effort cleanup
		}
	}
	delete process.env.GAME_DB_PATH;
	delete process.env.SESSION_HMAC_SECRET;
});

async function post(
	payload: unknown,
	headers: Record<string, string> = {},
): Promise<{ status: number; json: Record<string, unknown> | null }> {
	const body = typeof payload === "string" ? payload : JSON.stringify(payload);
	const request = new Request("http://localhost/api/game/actions", {
		method: "POST",
		headers: { "content-type": "application/json", ...headers },
		body,
	});
	const response = await POST(request);
	const json = (await response.json().catch(() => null)) as Record<
		string,
		unknown
	> | null;
	return { status: response.status, json };
}

describe("presence identity minting", () => {
	it("mints a server-signed session for a brand-new client", async () => {
		const { status, json } = await post({
			action: "presence",
			nickname: "Ann",
		});
		expect(status).toBe(200);
		const sessionId = json?.sessionId as string;
		const sig = json?.sig as string;
		expect(sessionId).toMatch(/^session_/);
		expect(verifySession(sessionId, sig, SECRET)).toBe(true);
	});

	it("keeps a client id that already proves ownership with a valid sig", async () => {
		const owned = "session_already_signed";
		const { status, json } = await post({
			action: "presence",
			sessionId: owned,
			nickname: "Ann",
			sig: signSession(owned, SECRET),
		});
		expect(status).toBe(200);
		expect(json?.sessionId).toBe(owned);
		expect(verifySession(owned, json?.sig as string, SECRET)).toBe(true);
	});

	it("mints a fresh id for a legacy (unsigned) localStorage session", async () => {
		const legacy = "session_legacy_client_123";
		const { status, json } = await post({
			action: "presence",
			sessionId: legacy,
			nickname: "Ann",
		});
		expect(status).toBe(200);
		// Unsigned id is untrusted: a new server id is returned, not the client's.
		expect(json?.sessionId).not.toBe(legacy);
		expect(json?.sessionId).toMatch(/^session_/);
		expect(
			verifySession(json?.sessionId as string, json?.sig as string, SECRET),
		).toBe(true);
	});

	it("never re-signs someone else's id (impersonation attempt)", async () => {
		const victim = "session_victim_abc";
		const { status, json } = await post({
			action: "presence",
			sessionId: victim,
			nickname: "Attacker",
		});
		expect(status).toBe(200);
		// Attacker gets a fresh id, and the returned sig does NOT authorize the victim's id.
		expect(json?.sessionId).not.toBe(victim);
		expect(verifySession(victim, json?.sig as string, SECRET)).toBe(false);
	});
});

describe("signature verification on identity actions", () => {
	it("rejects an identity action with no signature (401)", async () => {
		const { status } = await post({
			action: "requestVoteKick",
			sessionId: "s_unsigned",
			nickname: "Ann",
		});
		expect(status).toBe(401);
	});

	it("rejects a tampered signature (401)", async () => {
		// Valid sig for a different session, replayed against another sessionId.
		const sig = signSession("s_other", SECRET);
		const { status } = await post({
			action: "requestVoteKick",
			sessionId: "s_target",
			nickname: "Ann",
			sig,
		});
		expect(status).toBe(401);
	});

	it("accepts a validly signed identity action (not 401)", async () => {
		await post({ action: "ensure" });
		const sig = signSession("s_valid", SECRET);
		const { status } = await post({
			action: "requestVoteKick",
			sessionId: "s_valid",
			nickname: "Ann",
			sig,
		});
		// Auth passed — any remaining failure is a game-rule 400, never 401.
		expect(status).not.toBe(401);
	});

	it("accepts an upgraded legacy session end to end", async () => {
		// Legacy client presents an unsigned id; presence returns a fresh signed
		// identity, which the client then uses for an identity action.
		const presence = await post({
			action: "presence",
			sessionId: "session_upgraded_flow",
			nickname: "Ann",
		});
		const sessionId = presence.json?.sessionId as string;
		const sig = presence.json?.sig as string;
		const { status } = await post({
			action: "requestVoteKick",
			sessionId,
			nickname: "Ann",
			sig,
		});
		expect(status).not.toBe(401);
	});
});

describe("rate limiting", () => {
	it("returns 429 once the per-session budget is exhausted", async () => {
		const sessionId = "s_rate_limit";
		let limited = false;
		for (let i = 0; i < 31; i += 1) {
			const { status } = await post({
				action: "presence",
				sessionId,
				nickname: "Ann",
			});
			if (status === 429) {
				limited = true;
				break;
			}
		}
		expect(limited).toBe(true);
	});
});

describe("payload guards", () => {
	it("rejects an oversized body (413)", async () => {
		const huge = JSON.stringify({ action: "presence", pad: "x".repeat(9000) });
		const { status } = await post(huge);
		expect(status).toBe(413);
	});

	it("rejects a non-object JSON body (400)", async () => {
		const { status } = await post("[1,2,3]");
		expect(status).toBe(400);
	});

	it("rejects a missing action (400)", async () => {
		const { status } = await post({ nickname: "Ann" });
		expect(status).toBe(400);
	});
});
