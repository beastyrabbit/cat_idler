/**
 * SLOW, SEPARATE unaided-survival harness (NOT run in the default suite).
 *
 * WHY THIS EXISTS
 * The fast statistical test (serverSurvivalBalance.test.ts) can only measure the
 * DEMOGRAPHIC axis, because it feeds the colony. It cannot measure the resource
 * ECONOMY: job timers are real wall-clock (`endsAt = Date.now() + baseSec/timeScale`)
 * and `advanceTime` only moves `lastTick`, never real `now` — so in any fast
 * advanceTime loop hunts/water-runs NEVER complete and NEVER deliver, and a colony
 * that thrives live "starves" in the sim purely as an artifact. To measure the real
 * unaided economy you MUST let real wall-clock pass so jobs actually finish.
 *
 * WHAT IT DOES
 * Bootstraps N fresh colonies and runs each through the real `workerTick` with a
 * real ~1s tick spacing, fully unaided (no player supply, no god blessings), at a
 * balanced high timescale (`testTimeScale == testResourceDecayMultiplier`, so
 * production and consumption scale together as in normal play, but fast enough that
 * an 8h hunt lands in ~14 real-sec). Reports the fraction of colonies that survive
 * a multi-generation horizon without collapsing — the "unaided early survival"
 * number the design targets (>= ~90%).
 *
 * HOW TO RUN (it is skipped unless the env flag is set, because each colony takes
 * ~60-90 real seconds):
 *
 *     RUN_SURVIVAL_SIM=1 N_COLONIES=8 SIM_TICKS=90 \
 *       node_modules/.bin/vitest run tests/integration/survivalRealtime.harness.test.ts
 *
 * A separate CI job can run this on a schedule; the fast guard for the same fix
 * lives in serverSurvivalBalance.test.ts and runs in the normal suite.
 */

import { eq } from "drizzle-orm";
import { describe, expect, it } from "vitest";
import { createDb, type GameDb } from "@/db/client";
import { cats, colonies, runHistory } from "@/db/schema";
import { ensureGlobalColony, workerTick } from "@/server/game";

const ENABLED = process.env.RUN_SURVIVAL_SIM === "1";
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Balanced timescale: production:consumption ratio matches normal play, but an
 * 8h hunt completes in ~14 real-seconds so a full generation passes in ~1 min. */
const SIM_TIMESCALE = 2000;

async function runColony(seed: number, ticks: number): Promise<boolean> {
	const db: GameDb = createDb(":memory:");
	ensureGlobalColony(db);
	const cid = db
		.select()
		.from(colonies)
		.where(eq(colonies.isGlobal, true))
		.get()!._id;
	db.update(colonies)
		.set({
			worldSeed: seed,
			testTimeScale: SIM_TIMESCALE,
			testResourceDecayMultiplier: SIM_TIMESCALE,
			testResilienceHoursOverride: 0,
			testCriticalMsOverride: 10_000,
		})
		.where(eq(colonies._id, cid))
		.run();

	for (let i = 0; i < ticks; i++) {
		await sleep(1000);
		workerTick(db);
	}
	// Survived iff it never had to reset (runHistory records every collapse) and
	// still has living cats.
	const collapses = db.select().from(runHistory).all().length;
	const alive = db
		.select()
		.from(cats)
		.where(eq(cats.colonyId, cid))
		.all()
		.filter((c) => c.deathTime === null).length;
	return collapses === 0 && alive > 0;
}

describe.skipIf(!ENABLED)("unaided survival (real wall-clock harness)", () => {
	it(
		"an unaided early colony survives a multi-generation horizon >= 90% of the time",
		async () => {
			const N = Number(process.env.N_COLONIES ?? 10);
			const ticks = Number(process.env.SIM_TICKS ?? 90);
			let survived = 0;
			for (let i = 0; i < N; i++) {
				if (await runColony(4000 + i * 31, ticks)) survived++;
			}
			const rate = survived / N;
			// eslint-disable-next-line no-console
			console.log(
				`UNAIDED real-wall-clock survival: ${survived}/${N} (${(rate * 100).toFixed(0)}%) over ${((ticks * SIM_TIMESCALE) / 3600).toFixed(0)} game-hours each`,
			);
			expect(rate).toBeGreaterThanOrEqual(0.9);
		},
		30 * 60 * 1000,
	);
});
