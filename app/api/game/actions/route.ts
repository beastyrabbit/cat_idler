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
import { castVote, requestVoteKick } from "@/server/elections";
import {
	advanceTime,
	assignWorker,
	clickBoostJob,
	ensureGlobalState,
	type PlayerJobKind,
	planBuilding,
	purchaseUpgrade,
	requestJob,
	setTestAcceleration,
	setTestRngSeed,
	upsertPresence,
	workerTick,
} from "@/server/game";
import { createZone, removeZone } from "@/server/zones";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

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
	let body: Record<string, unknown>;
	try {
		body = await request.json();
	} catch {
		return NextResponse.json(
			{ ok: false, message: "Invalid JSON body." },
			{ status: 400 },
		);
	}

	const action = body.action;

	try {
		const db = getDb();

		switch (action) {
			case "ensure": {
				return NextResponse.json({ colonyId: ensureGlobalState(db) });
			}

			case "presence": {
				const sessionId = requireString(body.sessionId, "sessionId");
				const nickname = requireString(body.nickname, "nickname");
				return NextResponse.json({
					playerId: upsertPresence(db, sessionId, nickname),
				});
			}

			case "requestJob": {
				const sessionId = requireString(body.sessionId, "sessionId");
				const nickname = requireString(body.nickname, "nickname");
				const kind = body.kind as PlayerJobKind;
				if (!JOB_KINDS.includes(kind)) {
					throw new Error("Unknown job kind.");
				}
				return NextResponse.json(requestJob(db, { sessionId, nickname, kind }));
			}

			case "boost": {
				const sessionId = requireString(body.sessionId, "sessionId");
				const nickname = requireString(body.nickname, "nickname");
				const jobId = requireString(body.jobId, "jobId");
				return NextResponse.json(
					clickBoostJob(db, { sessionId, nickname, jobId }),
				);
			}

			case "purchaseUpgrade": {
				const sessionId = requireString(body.sessionId, "sessionId");
				const nickname = requireString(body.nickname, "nickname");
				const key = body.key as (typeof UPGRADE_KEYS)[number];
				if (!UPGRADE_KEYS.includes(key)) {
					throw new Error("Unknown upgrade key.");
				}
				return NextResponse.json(
					purchaseUpgrade(db, { sessionId, nickname, key }),
				);
			}

			case "castVote": {
				const sessionId = requireString(body.sessionId, "sessionId");
				const nickname = requireString(body.nickname, "nickname");
				const electionId = requireString(body.electionId, "electionId");
				const catId = requireString(body.catId, "catId");
				return NextResponse.json(
					castVote(db, { sessionId, nickname, electionId, catId }),
				);
			}

			case "requestVoteKick": {
				const sessionId = requireString(body.sessionId, "sessionId");
				const nickname = requireString(body.nickname, "nickname");
				return NextResponse.json(requestVoteKick(db, { sessionId, nickname }));
			}

			case "createZone": {
				const sessionId = requireString(body.sessionId, "sessionId");
				const nickname = requireString(body.nickname, "nickname");
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
				const sessionId = requireString(body.sessionId, "sessionId");
				const nickname = requireString(body.nickname, "nickname");
				const zoneId = requireString(body.zoneId, "zoneId");
				return NextResponse.json(
					removeZone(db, { sessionId, nickname, zoneId }),
				);
			}

			case "planBuilding": {
				const sessionId = requireString(body.sessionId, "sessionId");
				const nickname = requireString(body.nickname, "nickname");
				const type = body.type;
				if (type !== "workshop" && type !== "field") {
					throw new Error("Unknown building type.");
				}
				return NextResponse.json(
					planBuilding(db, { sessionId, nickname, type }),
				);
			}

			case "assignWorker": {
				const sessionId = requireString(body.sessionId, "sessionId");
				const nickname = requireString(body.nickname, "nickname");
				const catId = requireString(body.catId, "catId");
				const buildingId =
					body.buildingId === null
						? null
						: requireString(body.buildingId, "buildingId");
				return NextResponse.json(
					assignWorker(db, { sessionId, nickname, catId, buildingId }),
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
