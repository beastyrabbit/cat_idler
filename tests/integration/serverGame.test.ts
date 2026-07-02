/**
 * Integration tests for server/game.ts against an in-memory SQLite DB.
 *
 * These cover the simulation entry points that were previously only
 * exercisable through a running Convex deployment: bootstrap, jobs,
 * upgrades, click boosting, the worker tick, and run resets.
 */

import { and, eq } from "drizzle-orm";
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
	it("creates the global colony with 20 starter cats and 6 upgrades", () => {
		const colony = ensureGlobalColony(db);

		expect(colony.isGlobal).toBe(true);
		// Stocked storage scaled for 20 cats (~5h of food at base decay)
		expect(colony.resources).toEqual({
			food: 100,
			water: 100,
			herbs: 16,
			materials: 24,
			blessings: 0,
		});

		const dashboard = getGlobalDashboard(db)!;
		expect(dashboard.cats).toHaveLength(20);
		expect(dashboard.upgrades).toHaveLength(6);

		const names = new Set(
			dashboard.cats.map((cat: { name: string }) => cat.name),
		);
		expect(names.size).toBe(20);
	});

	it("founds a starter village: shrine, dens housing 10, and stocked storage", () => {
		ensureGlobalColony(db);
		const dashboard = getGlobalDashboard(db)!;

		const byType: Record<string, number> = {};
		for (const building of dashboard.buildings) {
			byType[building.type] = (byType[building.type] ?? 0) + 1;
		}

		expect(byType.shrine).toBe(1);
		expect(byType.den).toBe(5); // 2 cats per den -> housing for 10
		expect(byType.food_storage).toBe(1);

		// All starter buildings are finished and adjacent to the shrine
		for (const building of dashboard.buildings) {
			expect(building.constructionProgress).toBe(100);
			const dx = Math.abs(building.worldPosition.x - dashboard.anchor.x);
			const dy = Math.abs(building.worldPosition.y - dashboard.anchor.y);
			expect(Math.max(dx, dy)).toBeLessThanOrEqual(1);
		}
	});

	it("is idempotent", () => {
		const first = ensureGlobalState(db);
		const second = ensureGlobalState(db);
		expect(first).toBe(second);
		expect(getGlobalDashboard(db)!.cats).toHaveLength(20);
		expect(getGlobalDashboard(db)!.buildings).toHaveLength(7);
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
		expect(colony.resources.food).toBeLessThan(100);
		expect(colony.resources.water).toBeLessThan(100);
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
		expect(getGlobalDashboard(db)!.cats).toHaveLength(20);
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

/** Force a job's deadline into the past so the next elapsing tick completes it. */
function forceJobDue(database: GameDb, jobId: string) {
	database
		.update(jobs)
		.set({ endsAt: Date.now() - 1000 })
		.where(eq(jobs._id, jobId))
		.run();
}

function setResources(
	database: GameDb,
	colonyId: string,
	patch: Partial<{
		food: number;
		water: number;
		herbs: number;
		materials: number;
		blessings: number;
	}>,
) {
	const colony = ensureGlobalColony(database);
	database
		.update(colonies)
		.set({ resources: { ...colony.resources, ...patch } })
		.where(eq(colonies._id, colonyId))
		.run();
}

function eventMessages(database: GameDb): string[] {
	return (getGlobalDashboard(database)?.events ?? []).map(
		(event: { message: string }) => event.message,
	);
}

describe("upgrade persistence across run reset", () => {
	it("keeps upgrade levels, points, and blessings through a colony collapse", () => {
		const colony = ensureGlobalColony(db);

		db.update(colonies)
			.set({ globalUpgradePoints: 20 })
			.where(eq(colonies._id, colony._id))
			.run();
		purchaseUpgrade(db, { ...SESSION, key: "click_power" }); // cost 2
		purchaseUpgrade(db, { ...SESSION, key: "click_power" }); // cost 4
		setResources(db, colony._id, { food: 0, water: 0, blessings: 3 });
		db.update(cats)
			.set({ needs: { hunger: 0, thirst: 0, rest: 50, health: 1 } })
			.where(eq(cats.colonyId, colony._id))
			.run();

		advanceTime(db, 3600);
		const result = workerTick(db) as { reset?: boolean };
		expect(result.reset).toBe(true);

		const after = ensureGlobalColony(db);
		expect(after.globalUpgradePoints).toBe(20 - 2 - 4);
		expect(after.resources).toEqual({
			food: 100,
			water: 100,
			herbs: 16,
			materials: 24,
			blessings: 3,
		});

		const upgrade = getGlobalDashboard(db)?.upgrades.find(
			(u: { key: string }) => u.key === "click_power",
		);
		expect(upgrade?.level).toBe(2);
	});
});

describe("water crisis and recovery events", () => {
	it("logs the crisis headline when water crosses the threshold", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { food: 100, water: 4 });

		// 5 cats over an hour consume ~5 water — crosses 4 -> <= 3
		advanceTime(db, 3600);
		workerTick(db);

		expect(eventMessages(db)).toContain(
			"CRISIS: WATER RESERVES DANGEROUSLY LOW",
		);
	});

	it("logs recovery when a supply job lifts water back above safe levels", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { food: 100, water: 0 });

		const { jobId } = requestJob(db, { ...SESSION, kind: "supply_water" }) as {
			jobId: string;
		};
		workerTick(db); // promote queued -> active (sub-second tick is fine for promotion? no — needs elapsed)
		advanceTime(db, 2);
		workerTick(db);
		forceJobDue(db, jobId);
		advanceTime(db, 2);
		workerTick(db); // completes: water 0 -> ~8 (> 6, from <= 3)

		expect(eventMessages(db)).toContain(
			"Water reserves restored to safe levels.",
		);
	});
});

describe("build pipeline orchestration", () => {
	it("queues material gathering as a house prerequisite and pays out on completion", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { food: 100, water: 100, materials: 0 });
		// Seed chosen so the leader's policy-reliability roll passes for every
		// tier (second roll in the chain is 0.088; worst-case gate is 0.6).
		setTestRngSeed(db, 42);

		const { jobId } = requestJob(db, {
			...SESSION,
			kind: "leader_plan_house",
		}) as { jobId: string };
		forceJobDue(db, jobId);
		advanceTime(db, 2);
		workerTick(db);

		const planJob = db.select().from(jobs).where(eq(jobs._id, jobId)).get();
		expect(planJob?.status).toBe("completed");

		// Plenty of water but no materials -> planner queues the gather phase.
		const gather = db
			.select()
			.from(jobs)
			.where(and(eq(jobs.colonyId, colony._id), eq(jobs.kind, "build_house")))
			.all()
			.find(
				(job) =>
					(job.metadata as { phase?: string })?.phase === "gather_materials",
			);
		expect(gather).toBeDefined();
		expect((gather?.metadata as { reason?: string })?.reason).toBe(
			"house_prereq",
		);

		const materialsBefore = ensureGlobalColony(db).resources.materials;
		forceJobDue(db, gather!._id);
		advanceTime(db, 2);
		workerTick(db);

		expect(ensureGlobalColony(db).resources.materials).toBe(
			materialsBefore + 12,
		);
	});

	it("deducts resources and raises automation when a house is constructed", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { food: 100, water: 100, materials: 100 });

		const builder = getAliveCatsForTest(db, colony._id)[0];
		db.insert(jobs)
			.values({
				_id: "construct-test-job",
				colonyId: colony._id,
				kind: "build_house",
				status: "active",
				requestedByType: "leader",
				requestedByPlayerId: null,
				assignedCatId: builder._id,
				baseDurationSec: 10,
				speedMultiplier: 1,
				yieldMultiplier: 1,
				clickTimeReducedSec: 0,
				createdAt: Date.now(),
				startedAt: Date.now(),
				endsAt: Date.now() - 1000,
				metadata: { phase: "construct_house" },
			})
			.run();

		const before = ensureGlobalColony(db);
		advanceTime(db, 2);
		const result = workerTick(db) as {
			policyTier: "simple" | "normal" | "excellent";
			automationTier: number;
			resources: { water: number; materials: number };
		};

		const requirements = {
			simple: { water: 10, materials: 12 },
			normal: { water: 8, materials: 10 },
			excellent: { water: 6, materials: 8 },
		}[result.policyTier];

		expect(result.automationTier).toBeCloseTo(
			(before.automationTier ?? 0) + 0.05,
			5,
		);
		expect(result.resources.materials).toBe(100 - requirements.materials);
		// small consumption also drains water during the tick
		expect(result.resources.water).toBeLessThanOrEqual(
			100 - requirements.water,
		);
		expect(result.resources.water).toBeGreaterThan(
			100 - requirements.water - 1,
		);
	});
});

describe("unattended collapse", () => {
	it("resets the run when critical state persists past the threshold", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { food: 0, water: 0 });
		db.update(colonies)
			.set({
				lastPlayerActivityAt: Date.now() - 24 * 3600 * 1000,
				criticalSince: Date.now() - 10 * 60 * 1000,
			})
			.where(eq(colonies._id, colony._id))
			.run();

		advanceTime(db, 5);
		const result = workerTick(db) as { reset?: boolean };
		expect(result.reset).toBe(true);

		const history = db.select().from(runHistory).all();
		expect(history).toHaveLength(1);
		expect(history[0].reason).toBe("unattended-collapse");
	});
});

describe("ritual completion", () => {
	it("awards global upgrade points when a ritual job finishes", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { food: 100, water: 100 });
		const ritualist = getAliveCatsForTest(db, colony._id)[0];

		db.insert(jobs)
			.values({
				_id: "ritual-test-job",
				colonyId: colony._id,
				kind: "ritual",
				status: "active",
				requestedByType: "leader",
				requestedByPlayerId: null,
				assignedCatId: ritualist._id,
				baseDurationSec: 10,
				speedMultiplier: 1,
				yieldMultiplier: 1,
				clickTimeReducedSec: 0,
				createdAt: Date.now(),
				startedAt: Date.now(),
				endsAt: Date.now() - 1000,
				metadata: {},
			})
			.run();

		const pointsBefore = ensureGlobalColony(db).globalUpgradePoints ?? 0;
		advanceTime(db, 2);
		workerTick(db);

		// +1 base (ritual_mastery is level 0)
		expect(ensureGlobalColony(db).globalUpgradePoints).toBe(pointsBefore + 1);
	});
});

function getAliveCatsForTest(database: GameDb, colonyId: string) {
	return database
		.select()
		.from(cats)
		.where(eq(cats.colonyId, colonyId))
		.all()
		.filter((cat) => cat.deathTime === null);
}
