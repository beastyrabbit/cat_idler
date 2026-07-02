/**
 * Player/test actions endpoint.
 *
 * POST { action, ...payload }. Soft failures (e.g. "ritual already
 * pending") return 200 with { ok: false, message } — the client treats
 * them as inline notices. Game-rule rejections return 400 with their
 * message; unexpected internal errors are logged server-side and return
 * a generic 500.
 *
 * Test actions (advanceTime, setTestAcceleration, setTestRngSeed) are
 * only available outside production unless GAME_ENABLE_TEST_ACTIONS=1.
 */

import { NextResponse } from "next/server";

import { getDb } from "@/db/client";
import {
	issueSessionId,
	signSession,
	verifySession,
} from "@/lib/game/identity";
import { RateLimiter } from "@/lib/game/rateLimiter";
import { castVote, requestVoteKick } from "@/server/elections";
import {
	advanceTime,
	assignWorker,
	buildRoad,
	clickBoostJob,
	defendRaid,
	ensureGlobalState,
	type PlayerJobKind,
	planBuilding,
	purchaseUpgrade,
	requestJob,
	setTestAcceleration,
	setTestRngSeed,
	spawnRaidForTest,
	trainWarrior,
	unlockNode,
	upsertPresence,
	workerTick,
} from "@/server/game";
import { getSessionSecret } from "@/server/players";
import { createZone, removeZone } from "@/server/zones";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

/** Reject bodies larger than this before parsing (payloads here are tiny). */
const MAX_BODY_BYTES = 8 * 1024;

/** Per-session action budget: 30 actions per 10s window, else HTTP 429. */
const rateLimiter = new RateLimiter(30, 10_000);

/**
 * Actions that carry a player identity. `presence` mints/refreshes the signed
 * session; every other entry additionally requires a valid signature.
 */
const IDENTITY_ACTIONS = new Set([
	"presence",
	"requestJob",
	"boost",
	"purchaseUpgrade",
	"castVote",
	"requestVoteKick",
	"createZone",
	"removeZone",
	"planBuilding",
	"unlockNode",
	"assignWorker",
	"trainWarrior",
	"defendRaid",
	"buildRoad",
]);

/** Rejection for a missing/forged session signature (maps to HTTP 401). */
class SessionAuthError extends Error {}

/**
 * Validate identity fields and the session signature. Used by every
 * identity-bearing action except `presence` (which mints the signature).
 */
function requireSignedSession(body: Record<string, unknown>): {
	sessionId: string;
	nickname: string;
} {
	const sessionId = requireString(body.sessionId, "sessionId");
	const nickname = requireString(body.nickname, "nickname");
	const sig = typeof body.sig === "string" ? body.sig : null;
	if (!verifySession(sessionId, sig, getSessionSecret())) {
		throw new SessionAuthError(
			"Session signature missing or invalid. Refresh to re-establish your session.",
		);
	}
	return { sessionId, nickname };
}

const JOB_KINDS: PlayerJobKind[] = [
	"supply_food",
	"supply_water",
	"leader_plan_hunt",
	"leader_plan_house",
	"ritual",
];

const UPGRADE_KEYS = [
	"click_power",
	"supply_speed",
	"hunt_mastery",
	"build_mastery",
	"ritual_mastery",
	"resilience",
] as const;

const ACCELERATION_PRESETS = [
	"off",
	"fast",
	"turbo",
	"hyper",
	"ludicrous",
] as const;

/** Largest time skip a test may request (24h). */
const MAX_ADVANCE_SECONDS = 86_400;

function testActionsEnabled(): boolean {
	return (
		process.env.GAME_ENABLE_TEST_ACTIONS === "1" ||
		process.env.NODE_ENV !== "production"
	);
}

function requireString(value: unknown, name: string): string {
	if (typeof value !== "string" || value.length === 0) {
		throw new Error(`Missing or invalid ${name}.`);
	}
	return value;
}

/** Internal failures whose details must not reach the client. */
function isInternalError(err: unknown): boolean {
	if (err instanceof TypeError || err instanceof RangeError) {
		return true;
	}
	// better-sqlite3 throws SqliteError (name + SQLITE_* code)
	const candidate = err as { name?: string; code?: string };
	return (
		candidate?.name === "SqliteError" ||
		(typeof candidate?.code === "string" && candidate.code.startsWith("SQLITE"))
	);
}

function testActionsDisabledResponse() {
	return NextResponse.json(
		{ ok: false, message: "Test actions are disabled." },
		{ status: 404 },
	);
}

export async function POST(request: Request) {
	// Size guard before parsing — payloads here are tiny; anything large is abuse.
	const raw = await request.text();
	if (raw.length > MAX_BODY_BYTES) {
		return NextResponse.json(
			{ ok: false, message: "Request body too large." },
			{ status: 413 },
		);
	}

	let body: Record<string, unknown>;
	try {
		const parsed = JSON.parse(raw);
		if (
			typeof parsed !== "object" ||
			parsed === null ||
			Array.isArray(parsed)
		) {
			throw new Error("body must be a JSON object");
		}
		body = parsed as Record<string, unknown>;
	} catch {
		return NextResponse.json(
			{ ok: false, message: "Invalid JSON body." },
			{ status: 400 },
		);
	}

	const action = body.action;
	if (typeof action !== "string") {
		return NextResponse.json(
			{ ok: false, message: "Missing or invalid action." },
			{ status: 400 },
		);
	}

	// Per-session spam brake on identity-bearing actions.
	if (IDENTITY_ACTIONS.has(action) && typeof body.sessionId === "string") {
		if (!rateLimiter.check(body.sessionId)) {
			return NextResponse.json(
				{ ok: false, message: "Too many actions — slow down." },
				{ status: 429 },
			);
		}
	}

	try {
		const db = getDb();

		switch (action) {
			case "ensure": {
				return NextResponse.json({ colonyId: ensureGlobalState(db) });
			}

			case "presence": {
				const nickname = requireString(body.nickname, "nickname");
				const secret = getSessionSecret();
				const providedSession =
					typeof body.sessionId === "string" && body.sessionId.length > 0
						? body.sessionId
						: null;
				const providedSig = typeof body.sig === "string" ? body.sig : null;
				// Only keep the client's id if it already proves ownership with a
				// valid signature. An unsigned/forged id (including someone else's)
				// is untrusted — mint a fresh server-generated id and let the client
				// adopt what we return. This prevents re-signing a victim's id.
				const sessionId =
					providedSession && verifySession(providedSession, providedSig, secret)
						? providedSession
						: issueSessionId();
				const sig = signSession(sessionId, secret);
				return NextResponse.json({
					playerId: upsertPresence(db, sessionId, nickname),
					sessionId,
					sig,
				});
			}

			case "requestJob": {
				const { sessionId, nickname } = requireSignedSession(body);
				const kind = body.kind as PlayerJobKind;
				if (!JOB_KINDS.includes(kind)) {
					throw new Error("Unknown job kind.");
				}
				return NextResponse.json(requestJob(db, { sessionId, nickname, kind }));
			}

			case "boost": {
				const { sessionId, nickname } = requireSignedSession(body);
				const jobId = requireString(body.jobId, "jobId");
				return NextResponse.json(
					clickBoostJob(db, { sessionId, nickname, jobId }),
				);
			}

			case "purchaseUpgrade": {
				const { sessionId, nickname } = requireSignedSession(body);
				const key = body.key as (typeof UPGRADE_KEYS)[number];
				if (!UPGRADE_KEYS.includes(key)) {
					throw new Error("Unknown upgrade key.");
				}
				return NextResponse.json(
					purchaseUpgrade(db, { sessionId, nickname, key }),
				);
			}

			case "castVote": {
				const { sessionId, nickname } = requireSignedSession(body);
				const electionId = requireString(body.electionId, "electionId");
				const catId = requireString(body.catId, "catId");
				return NextResponse.json(
					castVote(db, { sessionId, nickname, electionId, catId }),
				);
			}

			case "requestVoteKick": {
				const { sessionId, nickname } = requireSignedSession(body);
				return NextResponse.json(requestVoteKick(db, { sessionId, nickname }));
			}

			case "createZone": {
				const { sessionId, nickname } = requireSignedSession(body);
				const kind = body.kind;
				if (kind !== "avoid" && kind !== "gather") {
					throw new Error("Unknown zone kind.");
				}
				const a = body.a as { x?: unknown; y?: unknown };
				const b = body.b as { x?: unknown; y?: unknown };
				const durationMs = Number(body.durationMs);
				if (
					typeof a?.x !== "number" ||
					typeof a?.y !== "number" ||
					typeof b?.x !== "number" ||
					typeof b?.y !== "number" ||
					!Number.isFinite(durationMs)
				) {
					throw new Error("Invalid zone rectangle.");
				}
				return NextResponse.json(
					createZone(db, {
						sessionId,
						nickname,
						kind,
						a: { x: a.x, y: a.y },
						b: { x: b.x, y: b.y },
						durationMs,
					}),
				);
			}

			case "removeZone": {
				const { sessionId, nickname } = requireSignedSession(body);
				const zoneId = requireString(body.zoneId, "zoneId");
				return NextResponse.json(
					removeZone(db, { sessionId, nickname, zoneId }),
				);
			}

			case "planBuilding": {
				const { sessionId, nickname } = requireSignedSession(body);
				const type = body.type;
				if (
					type !== "workshop" &&
					type !== "field" &&
					type !== "research_hut" &&
					type !== "school" &&
					type !== "smithy" &&
					type !== "barracks"
				) {
					throw new Error("Unknown building type.");
				}
				return NextResponse.json(
					planBuilding(db, { sessionId, nickname, type }),
				);
			}

			case "unlockNode": {
				const { sessionId, nickname } = requireSignedSession(body);
				const nodeId = requireString(body.nodeId, "nodeId");
				return NextResponse.json(
					unlockNode(db, { sessionId, nickname, nodeId }),
				);
			}

			case "assignWorker": {
				const { sessionId, nickname } = requireSignedSession(body);
				const catId = requireString(body.catId, "catId");
				const buildingId =
					body.buildingId === null
						? null
						: requireString(body.buildingId, "buildingId");
				return NextResponse.json(
					assignWorker(db, { sessionId, nickname, catId, buildingId }),
				);
			}

			case "trainWarrior": {
				const { sessionId, nickname } = requireSignedSession(body);
				const catId =
					body.catId == null ? null : requireString(body.catId, "catId");
				return NextResponse.json(
					trainWarrior(db, { sessionId, nickname, catId }),
				);
			}

			case "defendRaid": {
				const { sessionId, nickname } = requireSignedSession(body);
				return NextResponse.json(defendRaid(db, { sessionId, nickname }));
			}

			case "spawnRaid": {
				if (!testActionsEnabled()) {
					return testActionsDisabledResponse();
				}
				const count = body.count == null ? undefined : Number(body.count);
				const strength =
					body.strength == null ? undefined : Number(body.strength);
				return NextResponse.json(
					spawnRaidForTest(db, {
						atGate: body.atGate !== false,
						count,
						strength,
					}),
				);
			}

			case "buildRoad": {
				const { sessionId, nickname } = requireSignedSession(body);
				const a = body.a as { x?: unknown; y?: unknown };
				const b = body.b as { x?: unknown; y?: unknown };
				if (
					typeof a?.x !== "number" ||
					typeof a?.y !== "number" ||
					typeof b?.x !== "number" ||
					typeof b?.y !== "number"
				) {
					throw new Error("Invalid road endpoints.");
				}
				return NextResponse.json(
					buildRoad(db, {
						sessionId,
						nickname,
						a: { x: a.x, y: a.y },
						b: { x: b.x, y: b.y },
					}),
				);
			}

			case "setTestAcceleration": {
				if (!testActionsEnabled()) {
					return testActionsDisabledResponse();
				}
				const preset = body.preset as (typeof ACCELERATION_PRESETS)[number];
				if (!ACCELERATION_PRESETS.includes(preset)) {
					throw new Error("Unknown acceleration preset.");
				}
				return NextResponse.json(setTestAcceleration(db, preset));
			}

			case "advanceTime": {
				if (!testActionsEnabled()) {
					return testActionsDisabledResponse();
				}
				const seconds = Number(body.seconds);
				if (
					!Number.isFinite(seconds) ||
					seconds < 1 ||
					seconds > MAX_ADVANCE_SECONDS
				) {
					throw new Error(
						`Invalid seconds (must be 1..${MAX_ADVANCE_SECONDS}).`,
					);
				}
				const result = advanceTime(db, seconds);
				// Apply the skipped time immediately so tests see the effect
				// without waiting for the worker's next tick.
				workerTick(db);
				return NextResponse.json(result);
			}

			case "setTestRngSeed": {
				if (!testActionsEnabled()) {
					return testActionsDisabledResponse();
				}
				const seed = body.seed;
				if (seed !== null && typeof seed !== "number") {
					throw new Error("Invalid seed.");
				}
				return NextResponse.json(setTestRngSeed(db, seed as number | null));
			}

			default:
				return NextResponse.json(
					{ ok: false, message: "Unknown action." },
					{ status: 400 },
				);
		}
	} catch (err) {
		if (err instanceof SessionAuthError) {
			return NextResponse.json(
				{ ok: false, message: err.message },
				{ status: 401 },
			);
		}

		console.error(`[actions] ${String(action)} failed:`, err);

		if (isInternalError(err) || !(err instanceof Error)) {
			return NextResponse.json(
				{ ok: false, message: "Something went wrong on the server." },
				{ status: 500 },
			);
		}

		// Game-rule rejection (e.g. "Not enough ritual points.") — user-facing.
		return NextResponse.json(
			{ ok: false, message: err.message },
			{ status: 400 },
		);
	}
}
