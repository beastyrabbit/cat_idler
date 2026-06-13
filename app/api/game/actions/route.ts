/**
 * Player/test actions endpoint.
 *
 * POST { action, ...payload }. Soft failures (e.g. "ritual already
 * pending") return 200 with { ok: false, message } — the client treats
 * them as inline notices. Invalid input or unexpected errors return 4xx.
 */

import { NextResponse } from "next/server";

import { getDb } from "@/db/client";
import {
	advanceTime,
	clickBoostJob,
	ensureGlobalState,
	type PlayerJobKind,
	purchaseUpgrade,
	requestJob,
	setTestAcceleration,
	setTestRngSeed,
	upsertPresence,
	workerTick,
} from "@/server/game";

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

const ACCELERATION_PRESETS = ["off", "fast", "turbo"] as const;

function requireString(value: unknown, name: string): string {
	if (typeof value !== "string" || value.length === 0) {
		throw new Error(`Missing or invalid ${name}.`);
	}
	return value;
}

export async function POST(request: Request) {
	const db = getDb();

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

			case "setTestAcceleration": {
				const preset = body.preset as (typeof ACCELERATION_PRESETS)[number];
				if (!ACCELERATION_PRESETS.includes(preset)) {
					throw new Error("Unknown acceleration preset.");
				}
				return NextResponse.json(setTestAcceleration(db, preset));
			}

			case "advanceTime": {
				const seconds = Number(body.seconds);
				if (!Number.isFinite(seconds)) {
					throw new Error("Invalid seconds.");
				}
				const result = advanceTime(db, seconds);
				// Apply the skipped time immediately so tests see the effect
				// without waiting for the worker's next tick.
				workerTick(db);
				return NextResponse.json(result);
			}

			case "setTestRngSeed": {
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
		const message = err instanceof Error ? err.message : "Action failed.";
		return NextResponse.json({ ok: false, message }, { status: 400 });
	}
}
