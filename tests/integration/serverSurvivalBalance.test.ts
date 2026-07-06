/**
 * Statistical survival-balance regression tests.
 *
 * The user report was "the village dies on raids". Investigation of the live
 * DB (572 logged collapses) showed raids were NOT the primary killer — the
 * raid director already caps a lost raid at one casualty and grants a grace
 * window, so a raid only finishes a colony that is already down to its last
 * cat. The real killer was an old-age COHORT collapse:
 *
 *   1. `starterAgeHours` bunched the founding roster into two ages (12h, 29h),
 *      so the whole colony crossed the 48h old-age mortality cliff together in
 *      two die-off waves instead of a steady trickle.
 *   2. The 20-cat roster sat far above the founding housing cap (14), and
 *      `colonyCanBreed` blocks conception while population >= capacity — so the
 *      founders aged out with ZERO replacement births and the colony reliably
 *      collapsed of old age.
 *
 * The fixes fan the founders evenly across the young/adult band and enlarge the
 * founding dens (level 2 longhouses) so the colony sits just under its housing
 * cap and breeds as elders die — without changing the village's building count.
 *
 * These tests bootstrap many colonies, seed each deterministically, and simulate
 * them over a multi-generation horizon, asserting:
 *   - a fed EARLY colony replaces its founders and does not demographically
 *     collapse (the design's ~10% "unaided early" fragility is economy-driven and
 *     lives in the live worker, not this fed harness — see the fidelity note);
 *   - a MEDIUM village is robust (~1%);
 *   - raids are essentially never the finishing blow on their own.
 *
 * Fidelity note: the colonies are kept fed (stores topped up each step) so the
 * test isolates the demographic + raid subsystems, whose effects scale linearly
 * with elapsed game-time and are therefore faithful under the coarse `advanceTime`
 * steps that keep this test fast. The resource economy itself is job-driven
 * (hunts, water runs) and completes at most once per tick, so it cannot be
 * simulated faithfully by large time jumps — it is covered by the fine-grained
 * live worker instead. Every collapse observed here is thus attributable to
 * demographics or raids, which is exactly what these targets govern.
 */

import { and, eq } from "drizzle-orm";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import { createDb, type GameDb } from "@/db/client";
import { buildings, cats, colonies, events, runHistory } from "@/db/schema";
import { BREEDING_FOOD_PER_CAT } from "@/lib/game/lifeSim";
import {
	advanceTime,
	ensureGlobalColony,
	getGlobalDashboard,
	setTestRngSeed,
	workerTick,
} from "@/server/game";

// Deterministic row ids. Cat/building ids come from nanoid (crypto-random), and
// cat iteration order (keyed by id) decides which cat consumes which seeded
// life/raid roll — so random ids leak nondeterminism into the collapse counts.
// A monotonic id keeps the whole harness reproducible; nanoid ids are opaque
// tokens here, so sequential ones serve identically.
const { nextId } = vi.hoisted(() => {
	let n = 0;
	return {
		nextId: () => `id-${(n++).toString().padStart(12, "0")}`,
	};
});
vi.mock("nanoid", () => ({ nanoid: () => nextId() }));

// The simulation's seeded RNG chain (setTestRngSeed) makes the life/raid rolls
// reproducible, but createStarterCats rolls cat *stats* off bare Math.random,
// which feeds raid defense and would otherwise make these collapse counts vary
// run-to-run. Pin Math.random to a deterministic stream so the whole harness —
// and therefore the asserted rates — is stable.
let realRandom: () => number;
let rngState = 0x2545f491;
function seededRandom(): number {
	// mulberry32
	rngState |= 0;
	rngState = (rngState + 0x6d2b79f5) | 0;
	let t = Math.imul(rngState ^ (rngState >>> 15), 1 | rngState);
	t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
	return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
}
beforeAll(() => {
	realRandom = Math.random;
	Math.random = seededRandom;
});
afterAll(() => {
	Math.random = realRandom;
});

type Scenario = "early" | "medium";

/** Coarse but demographically-faithful step (see fidelity note above). */
const STEP_GAME_HOURS = 2;
/** ~2.5 cat lifetimes — long enough for several generations to turn over. */
const HORIZON_GAME_HOURS = 140;
/** Kept above consumption every step so the economy is never the cause of death. */
const FED_STORE = 400;

interface RunResult {
	collapsed: boolean;
	raidWipeouts: number;
	raidsSeen: number;
}

function bootstrapEarly(seed: number): GameDb {
	const db = createDb(":memory:");
	ensureGlobalColony(db);
	setTestRngSeed(db, seed);
	// worldSeed defaults to Date.now(); pin it so terrain (and therefore raid
	// pathing/timing) is reproducible across runs.
	db.update(colonies)
		.set({ worldSeed: seed })
		.where(eq(colonies.isGlobal, true))
		.run();
	return db;
}

function bootstrapMedium(seed: number): GameDb {
	const db = bootstrapEarly(seed);
	const colony = db
		.select()
		.from(colonies)
		.where(eq(colonies.isGlobal, true))
		.get();
	if (!colony) throw new Error("no colony");

	// Grow to a "medium village": extra completed dens (housing headroom +
	// village level) and a larger, age-staggered roster (~34 cats), as if the
	// colony had matured past its founding.
	for (let i = 0; i < 14; i++) {
		db.insert(buildings)
			.values({
				_id: nextId(),
				colonyId: colony._id,
				type: "den",
				level: 2,
				position: { x: 100 + i, y: 100 },
				constructionProgress: 100,
			})
			.run();
	}
	for (let i = 0; i < 14; i++) {
		db.insert(cats)
			.values({
				_id: nextId(),
				colonyId: colony._id,
				name: `Settler ${i}`,
				parentIds: [null, null],
				birthTime: Date.now(),
				ageHours: 10 + ((i * 31) % 34),
				pregnancyDueAgeHours: null,
				pregnancyMateId: null,
				deathTime: null,
				stats: {
					attack: 45,
					defense: 45,
					hunting: 45,
					medicine: 35,
					cleaning: 40,
					building: 40,
					leadership: 40,
					vision: 45,
				},
				needs: { hunger: 100, thirst: 100, rest: 100, health: 100 },
				currentTask: null,
				position: { map: "colony", x: 6, y: 6 },
				isPregnant: false,
				pregnancyDueTime: null,
				spriteParams: {},
				specialization: i % 4 === 0 ? "warrior" : null,
				roleXp: { hunter: 0, architect: 0, ritualist: 0, warrior: 0 },
			})
			.run();
	}
	return db;
}

function simulate(db: GameDb, ageForRaids: boolean): RunResult {
	const stepSec = Math.round(STEP_GAME_HOURS * 3600);
	const steps = Math.round(HORIZON_GAME_HOURS / STEP_GAME_HOURS);
	const colonyId = db
		.select()
		.from(colonies)
		.where(eq(colonies.isGlobal, true))
		.get()!._id;
	for (let s = 0; s < steps; s++) {
		const colony = db
			.select()
			.from(colonies)
			.where(eq(colonies.isGlobal, true))
			.get();
		if (!colony) throw new Error("no colony");
		// Keep the larder full so the economy is never the cause of death, and pin
		// runStartedAt so raid timing is deterministic. The raid director's
		// colonyAgeSec is measured from wall-clock (now - runStartedAt), which a
		// fast test loop never advances — so we drive it explicitly. For the
		// established MEDIUM village we recede runStartedAt in lockstep with
		// advanceTime so colonyAgeSec climbs past the 8h grace and raids actually
		// fire (a matured settlement should be raided). For the EARLY colony we hold
		// runStartedAt at `now` so colonyAgeSec stays ~0, firmly inside grace: a
		// fresh unaided colony is young and rarely raided, so its rate reflects
		// demographic fragility rather than raids inflated by the fed larder's loot.
		// Pinning both (rather than leaving runStartedAt at bootstrap) also removes
		// the wall-clock drift that made the result vary with suite load.
		db.update(colonies)
			.set({
				resources: { ...colony.resources, food: FED_STORE, water: FED_STORE },
				runStartedAt: ageForRaids
					? (colony.runStartedAt ?? colony.createdAt) - stepSec * 1000
					: Date.now(),
			})
			.where(eq(colonies._id, colony._id))
			.run();
		advanceTime(db, stepSec);
		workerTick(db);
	}
	const history = db.select().from(runHistory).all();
	const raidsSeen = db
		.select()
		.from(events)
		.where(and(eq(events.colonyId, colonyId), eq(events.type, "raid_incoming")))
		.all().length;
	return {
		collapsed: history.length > 0,
		raidWipeouts: history.filter((h) => h.reason === "raid-wipeout").length,
		raidsSeen,
	};
}

function measure(scenario: Scenario, n: number) {
	// Fixed starting point so each test is reproducible regardless of run order.
	rngState = 0x2545f491;
	let collapsed = 0;
	let raidWipeouts = 0;
	let raidsSeen = 0;
	for (let i = 0; i < n; i++) {
		const seed = 1000 + i * 7;
		const db =
			scenario === "early" ? bootstrapEarly(seed) : bootstrapMedium(seed);
		const r = simulate(db, scenario === "medium");
		if (r.collapsed) collapsed++;
		raidWipeouts += r.raidWipeouts;
		raidsSeen += r.raidsSeen;
	}
	return { collapsed, raidWipeouts, raidsSeen, rate: collapsed / n };
}

describe("survival balance (statistical)", () => {
	const N = 24;

	it("a fed early colony replaces its founders and does not demographically collapse", () => {
		const { rate } = measure("early", N);
		// This measures the DEMOGRAPHIC axis only (the colony is fed). Pre-fix the
		// founders bunched into two ages and crossed the 48h old-age cliff together,
		// collapsing 75-100% here; with the even age spread they die in a trickle the
		// breeding replaces, so a fed early colony holds. The ~10% "unaided early"
		// fragility the design targets is NOT visible here — it lives in the resource
		// economy (subsistence food keeps breeding below its threshold), which the
		// coarse fed harness cannot model; that is covered by the live worker.
		expect(rate).toBeLessThanOrEqual(0.1);
	}, 120_000);

	it("a medium village survives many raids across the horizon, which never finish it", () => {
		const { rate, raidWipeouts, raidsSeen } = measure("medium", N);
		// The village must actually be raided for this to mean anything — the
		// harness ages runStartedAt so raids clear the grace window and fire.
		expect(raidsSeen).toBeGreaterThan(0);
		// Design target ~1%. A matured village should almost never collapse...
		expect(rate).toBeLessThanOrEqual(0.05);
		// ...and raids specifically must never be the finishing blow on their own.
		expect(raidWipeouts).toBe(0);
	}, 120_000);
});

/**
 * Fast structural guard for the unaided-survival fix (the economy fragility the
 * statistical test above cannot see — see its comment). A fresh colony must be
 * able to breed a replacement generation from turn one; the real survival number
 * is measured by the slow real-wall-clock harness (survivalRealtime.harness.test.ts).
 * These cheap invariants catch a regression of the fix without running a sim.
 */
describe("unaided-survival fix invariants (fast guard)", () => {
	it("a fresh colony opens breeding-ready: housing headroom, food surplus, young founders", () => {
		const db = createDb(":memory:");
		const colony = ensureGlobalColony(db);
		const dash = getGlobalDashboard(db)!;
		const founders = db
			.select()
			.from(cats)
			.where(eq(cats.colonyId, colony._id))
			.all()
			.filter((c) => c.deathTime === null);

		// Housing seats ABOVE the roster, so conception is not blocked while the
		// founders are still fertile adults (the demographic trap).
		expect(dash.housing.capacity).toBeGreaterThan(founders.length);

		// The starting food buffer clears the per-capita breeding floor for the
		// whole roster, so breeding fires immediately rather than after the economy
		// spins up (and it bridges the 8h gap to the first hunt payout).
		expect(colony.resources.food).toBeGreaterThanOrEqual(
			founders.length * BREEDING_FOOD_PER_CAT,
		);

		// Every founder is a working-age youngster with a full adult breeding
		// window (24-48h) ahead of it — none parked near the 48h old-age cliff.
		for (const cat of founders) {
			expect(cat.ageHours ?? 0).toBeLessThanOrEqual(30);
			expect(cat.ageHours ?? 0).toBeGreaterThanOrEqual(6);
		}
	});
});
