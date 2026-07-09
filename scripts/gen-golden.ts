/**
 * Golden-master fixture generator (P0.7).
 *
 * Drives the *existing TypeScript simulation* headlessly and emits per-tick
 * aggregate snapshots that the Rust port (`cat-sim`) is validated against.
 *
 * Determinism recipe (mirrors tests/integration/serverGame.test.ts):
 *   - Math.random is replaced with a seeded mulberry32 PRNG.
 *   - worldSeed is pinned and the terrain regenerated from it.
 *   - the tick RNG is pinned via setTestRngSeed.
 * Because the game is only behaviourally deterministic (a few cosmetic paths use
 * raw Math.random, and cat ids come from nanoid which affects assignment
 * tie-breaks), the fixtures capture AGGREGATE state (totals / counts / bands),
 * NOT per-entity ids. Rust parity is judged on trajectory shape, per the
 * "same idea" fidelity bar — not bit-identical output.
 *
 * Run:  bun run scripts/gen-golden.ts [--seed N] [--ticks N] [--step-sec N] [--out PATH]
 */
import { eq } from "drizzle-orm";
import { createDb } from "@/db/client";
import { colonies, worldTiles } from "@/db/schema";
import {
	advanceTime,
	ensureGlobalColony,
	getGlobalDashboard,
	setTestRngSeed,
	workerTick,
} from "@/server/game";
import { initializeWorldMap } from "@/server/worldMap";

function mulberry32(seed: number): () => number {
	let a = seed | 0;
	return () => {
		a = (a + 0x6d2b79f5) | 0;
		let t = Math.imul(a ^ (a >>> 15), 1 | a);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

function arg(name: string, fallback: number): number {
	const i = process.argv.indexOf(`--${name}`);
	return i >= 0 && process.argv[i + 1] ? Number(process.argv[i + 1]) : fallback;
}
function argStr(name: string, fallback: string): string {
	const i = process.argv.indexOf(`--${name}`);
	return i >= 0 && process.argv[i + 1] ? String(process.argv[i + 1]) : fallback;
}

const SEED = arg("seed", 1234);
const TICKS = arg("ticks", 120);
const STEP_SEC = arg("step-sec", 60);
const WORLD_SEED = arg("world-seed", 20240703);
const OUT = argStr(
	"out",
	`docs/migration/fixtures/worker-tick-seed${SEED}-t${TICKS}.json`,
);

type Snapshot = {
	tick: number;
	elapsedSec: number;
	status: string;
	population: number;
	resources: Record<string, number>;
	jobsByKind: Record<string, number>;
	catsByActivity: Record<string, number>;
	threatPressure: number;
	threatBand: string;
	villageLevel: number;
	buildings: number;
};

function snapshot(db: ReturnType<typeof createDb>, tick: number): Snapshot {
	const d = getGlobalDashboard(db);
	if (!d) throw new Error("no dashboard");
	const jobsByKind: Record<string, number> = {};
	for (const j of d.jobs ?? [])
		jobsByKind[j.kind] = (jobsByKind[j.kind] ?? 0) + 1;
	const catsByActivity: Record<string, number> = {};
	for (const c of d.cats ?? []) {
		const a = (c as { activity?: string }).activity ?? "idle";
		catsByActivity[a] = (catsByActivity[a] ?? 0) + 1;
	}
	const res = (d.colony?.resources ?? {}) as Record<string, number>;
	const rounded: Record<string, number> = {};
	for (const [k, v] of Object.entries(res))
		rounded[k] = Math.round((v as number) * 100) / 100;
	return {
		tick,
		elapsedSec: tick * STEP_SEC,
		status: String(d.colony?.status ?? "unknown"),
		population: d.cats?.length ?? 0,
		resources: rounded,
		jobsByKind,
		catsByActivity,
		threatPressure: Math.round((d.threat?.pressure ?? 0) * 100) / 100,
		threatBand: String(d.threat?.band ?? "calm"),
		villageLevel: d.housing?.villageLevel ?? 1,
		buildings: d.buildings?.length ?? 0,
	};
}

function main() {
	const realRandom = Math.random;
	Math.random = mulberry32(SEED);
	try {
		const db = createDb(":memory:");
		const colony = ensureGlobalColony(db);
		db.update(colonies)
			.set({ worldSeed: WORLD_SEED })
			.where(eq(colonies._id, colony._id))
			.run();
		db.delete(worldTiles).where(eq(worldTiles.colonyId, colony._id)).run();
		initializeWorldMap(db, colony._id);
		setTestRngSeed(db, SEED);

		const snaps: Snapshot[] = [];
		for (let t = 1; t <= TICKS; t++) {
			advanceTime(db, STEP_SEC);
			workerTick(db);
			snaps.push(snapshot(db, t));
		}

		const out = {
			meta: {
				generator: "scripts/gen-golden.ts",
				seed: SEED,
				worldSeed: WORLD_SEED,
				ticks: TICKS,
				stepSec: STEP_SEC,
				note: "Aggregate behavioural fixture; ids excluded (nanoid). Rust parity = trajectory shape.",
			},
			snapshots: snaps,
		};
		const fs = require("node:fs") as typeof import("node:fs");
		const path = require("node:path") as typeof import("node:path");
		fs.mkdirSync(path.dirname(OUT), { recursive: true });
		fs.writeFileSync(OUT, `${JSON.stringify(out, null, 2)}\n`);
		const last = snaps[snaps.length - 1];
		console.log(
			`wrote ${OUT} (${snaps.length} ticks) — final: status=${last.status} pop=${last.population} food=${last.resources.food} threat=${last.threatBand}`,
		);
	} finally {
		Math.random = realRandom;
	}
}

main();
