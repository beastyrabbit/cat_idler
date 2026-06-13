/**
 * Integration tests for server/game.ts against an in-memory SQLite DB.
 *
 * These cover the simulation entry points that were previously only
 * exercisable through a running Convex deployment: bootstrap, jobs,
 * upgrades, click boosting, the worker tick, and run resets.
 */

import { eq } from "drizzle-orm";
import { beforeEach, describe, expect, it } from "vitest";

import { createDb, type GameDb } from "@/db/client";
import { cats, colonies, jobs, runHistory } from "@/db/schema";
import {
	advanceTime,
	clickBoostJob,
	ensureGlobalColony,
	ensureGlobalState,
	getGlobalDashboard,
	purchaseUpgrade,
	requestJob,
	setTestAcceleration,
	setTestRngSeed,
	workerTick,
} from "@/server/game";
import { upsertPlayer } from "@/server/players";

const SESSION = { sessionId: "session_test_1", nickname: "Tester" };

let db: GameDb;

beforeEach(() => {
	db = createDb(":memory:");
});

describe("bootstrap", () => {
	it("creates the global colony with 5 starter cats and 6 upgrades", () => {
		const colony = ensureGlobalColony(db);

		expect(colony.isGlobal).toBe(true);
		expect(colony.resources).toEqual({
			food: 24,
			water: 24,
			herbs: 8,
			materials: 0,
			blessings: 0,
		});

		const dashboard = getGlobalDashboard(db)!;
		expect(dashboard.cats).toHaveLength(5);
		expect(dashboard.upgrades).toHaveLength(6);
	});

	it("is idempotent", () => {
		const first = ensureGlobalState(db);
		const second = ensureGlobalState(db);
		expect(first).toBe(second);
		expect(getGlobalDashboard(db)!.cats).toHaveLength(5);
	});
});

describe("workerTick", () => {
	it("skips sub-second ticks", () => {
		ensureGlobalColony(db);
		const result = workerTick(db) as { ok: boolean; skipped?: boolean };
		expect(result.ok).toBe(true);
		expect(result.skipped).toBe(true);
	});

	it("consumes resources and assigns a leader after elapsed time", () => {
		ensureGlobalColony(db);
		advanceTime(db, 60);

		const result = workerTick(db) as {
			ok: boolean;
			resources: { food: number; water: number };
		};
		expect(result.ok).toBe(true);

		const colony = ensureGlobalColony(db);
		expect(colony.leaderId).not.toBeNull();
		expect(colony.resources.food).toBeLessThan(24);
		expect(colony.resources.water).toBeLessThan(24);
	});

	it("chains the seeded RNG deterministically across ticks", () => {
		ensureGlobalColony(db);
		setTestRngSeed(db, 1234);

		advanceTime(db, 30);
		workerTick(db);

		const after = ensureGlobalColony(db);
		expect(after.testRngSeed).not.toBe(1234);
		expect(typeof after.testRngSeed).toBe("number");
	});
});

describe("requestJob", () => {
	it("queues a supply job and completes it after its duration", () => {
		ensureGlobalColony(db);

		const result = requestJob(db, { ...SESSION, kind: "supply_food" }) as {
			jobId: string;
		};
		expect(result.jobId).toBeTruthy();

		const queued = db
			.select()
			.from(jobs)
			.where(eq(jobs._id, result.jobId))
			.get()!;
		expect(queued.status).toBe("queued");
		expect(queued.kind).toBe("supply_food");

		const foodBefore = ensureGlobalColony(db).resources.food;

		// Jump past the job's end and tick (advanceTime + a tick that elapses).
		advanceTime(db, queued.baseDurationSec + 5);
		db.update(jobs)
			.set({ endsAt: Date.now() - 1000 })
			.where(eq(jobs._id, queued._id))
			.run();
		workerTick(db); // promotes queued -> active
		workerTick(db); // completes due job — but needs elapsed time again
		advanceTime(db, 2);
		workerTick(db);

		const completed = db
			.select()
			.from(jobs)
			.where(eq(jobs._id, result.jobId))
			.get()!;
		expect(completed.status).toBe("completed");

		const foodAfter = ensureGlobalColony(db).resources.food;
		expect(foodAfter).toBeGreaterThan(foodBefore - 1); // +8 supply minus small consumption
	});

	it("rejects duplicate strategic jobs", () => {
		ensureGlobalColony(db);

		const first = requestJob(db, { ...SESSION, kind: "leader_plan_hunt" }) as {
			jobId?: string;
		};
		expect(first.jobId).toBeTruthy();

		const second = requestJob(db, { ...SESSION, kind: "leader_plan_hunt" }) as {
			ok?: boolean;
			reason?: string;
		};
		expect(second.ok).toBe(false);
		expect(second.reason).toBe("already_in_progress");
	});

	it("marks ritual requests pending instead of queueing immediately", () => {
		ensureGlobalColony(db);

		const result = requestJob(db, { ...SESSION, kind: "ritual" }) as {
			requested?: boolean;
		};
		expect(result.requested).toBe(true);

		const colony = ensureGlobalColony(db);
		expect(colony.ritualRequestedAt).not.toBeNull();

		const repeat = requestJob(db, { ...SESSION, kind: "ritual" }) as {
			ok?: boolean;
		};
		expect(repeat.ok).toBe(false);
	});
});

describe("clickBoostJob", () => {
	it("reduces the job end time and tracks player clicks", () => {
		ensureGlobalColony(db);
		const { jobId } = requestJob(db, { ...SESSION, kind: "supply_food" }) as {
			jobId: string;
		};

		const before = db.select().from(jobs).where(eq(jobs._id, jobId)).get()!;
		const result = clickBoostJob(db, { ...SESSION, jobId }) as {
			reducedBySec: number;
			newEndsAt: number;
		};

		expect(result.reducedBySec).toBeGreaterThan(0);
		expect(result.newEndsAt).toBeLessThanOrEqual(before.endsAt);

		const after = db.select().from(jobs).where(eq(jobs._id, jobId)).get()!;
		expect(after.status).toBe("active");
		expect(after.clickTimeReducedSec).toBe(result.reducedBySec);
	});

	it("rejects boosts on unknown jobs", () => {
		ensureGlobalColony(db);
		expect(() => clickBoostJob(db, { ...SESSION, jobId: "missing" })).toThrow(
			/cannot be boosted/,
		);
	});
});

describe("purchaseUpgrade", () => {
	it("rejects purchases without enough ritual points", () => {
		ensureGlobalColony(db);
		expect(() =>
			purchaseUpgrade(db, { ...SESSION, key: "click_power" }),
		).toThrow(/Not enough ritual points/);
	});

	it("levels the upgrade and deducts points", () => {
		const colony = ensureGlobalColony(db);
		db.update(colonies)
			.set({ globalUpgradePoints: 10 })
			.where(eq(colonies._id, colony._id))
			.run();

		const result = purchaseUpgrade(db, { ...SESSION, key: "click_power" }) as {
			level: number;
			remainingPoints: number;
		};

		expect(result.level).toBe(1);
		expect(result.remainingPoints).toBe(10 - 2); // baseCost 2, level 0
	});

	it("rejects maxed upgrades", () => {
		const colony = ensureGlobalColony(db);
		db.update(colonies)
			.set({ globalUpgradePoints: 10_000 })
			.where(eq(colonies._id, colony._id))
			.run();

		for (let i = 0; i < 20; i++) {
			purchaseUpgrade(db, { ...SESSION, key: "click_power" });
		}

		expect(() =>
			purchaseUpgrade(db, { ...SESSION, key: "click_power" }),
		).toThrow(/already maxed/);
	});
});

describe("run reset", () => {
	it("records run history and starts a new run when all cats starve mid-tick", () => {
		const colony = ensureGlobalColony(db);

		// Drain the colony and push every cat to the brink so the survival
		// pass kills them during the tick (cats dead *before* the tick would
		// be resurrected by the bootstrap guard instead).
		db.update(colonies)
			.set({ resources: { ...colony.resources, food: 0, water: 0 } })
			.where(eq(colonies._id, colony._id))
			.run();
		db.update(cats)
			.set({ needs: { hunger: 0, thirst: 0, rest: 50, health: 1 } })
			.where(eq(cats.colonyId, colony._id))
			.run();

		advanceTime(db, 3600);
		workerTick(db);

		const after = ensureGlobalColony(db);
		expect(after.runNumber).toBe((colony.runNumber ?? 1) + 1);
		expect(after.status).toBe("starting");

		const history = db.select().from(runHistory).all();
		expect(history).toHaveLength(1);
		expect(history[0].reason).toBe("all-cats-dead");

		// resetGlobalRun seeds fresh starter cats when none survive
		expect(getGlobalDashboard(db)!.cats).toHaveLength(5);
	});
});

describe("test controls", () => {
	it("applies acceleration presets", () => {
		ensureGlobalColony(db);
		setTestAcceleration(db, "turbo");

		const colony = ensureGlobalColony(db);
		expect(colony.testTimeScale).toBeGreaterThan(1);
	});

	it("advanceTime moves lastTick backwards", () => {
		const before = ensureGlobalColony(db);
		advanceTime(db, 120);
		const after = ensureGlobalColony(db);
		expect(after.lastTick).toBe(before.lastTick - 120_000);
	});
});

describe("presence", () => {
	it("counts players seen in the online window", () => {
		ensureGlobalColony(db);
		upsertPlayer(db, "s1", "A");
		upsertPlayer(db, "s2", "B");
		upsertPlayer(db, "s2", "B renamed");

		const dashboard = getGlobalDashboard(db)!;
		expect(dashboard.onlineCount).toBe(2);
	});
});
