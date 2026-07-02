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
import {
	buildings as buildingsTable,
	cats,
	colonies,
	elections,
	jobs,
	runHistory,
	zones as zonesTable,
} from "@/db/schema";
import { castVote, requestVoteKick } from "@/server/elections";
import {
	advanceTime,
	assignWorker,
	clickBoostJob,
	ensureGlobalColony,
	ensureGlobalState,
	getGlobalDashboard,
	planBuilding,
	purchaseUpgrade,
	requestJob,
	setTestAcceleration,
	setTestRngSeed,
	workerTick,
} from "@/server/game";
import { upsertPlayer } from "@/server/players";
import { createZone, removeZone } from "@/server/zones";

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
			refined: 0,
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
			refined: 0,
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

		// The blessing is carried to the shrine first (Phase 6) — walk the
		// ritualist home across a few ticks.
		for (
			let i = 0;
			i < 10 &&
			(ensureGlobalColony(db).globalUpgradePoints ?? 0) === pointsBefore;
			i++
		) {
			advanceTime(db, 10);
			workerTick(db);
		}

		// +1 base (ritual_mastery is level 0)
		expect(ensureGlobalColony(db).globalUpgradePoints).toBe(pointsBefore + 1);
	});
});

describe("shrine deposits", () => {
	it("hunters carry the catch home and deposit it at the shrine", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { food: 100, water: 100 });
		const hunter = getAliveCatsForTest(db, colony._id)[0];

		// Far out in the field when the hunt wraps up.
		db.update(cats)
			.set({ position: { map: "world", x: 6, y: 26 } })
			.where(eq(cats._id, hunter._id))
			.run();

		db.insert(jobs)
			.values({
				_id: "hunt-deposit-job",
				colonyId: colony._id,
				kind: "hunt_expedition",
				status: "active",
				requestedByType: "leader",
				requestedByPlayerId: null,
				assignedCatId: hunter._id,
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

		const foodBefore = ensureGlobalColony(db).resources.food;
		advanceTime(db, 2);
		workerTick(db);

		// Yield is on the cat's back, not in the pantry.
		let carrier = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === hunter._id,
		)!;
		expect(carrier.carrying).not.toBeNull();
		const carried = carrier.carrying!.amount;
		expect(ensureGlobalColony(db).resources.food).toBeLessThan(
			foodBefore + carried,
		);
		expect(carrier.activity).toBe("returning");
		expect(carrier.destination).toEqual({ map: "world", x: 6, y: 6 });

		// Walk home (20 tiles at 0.5/s) and deposit.
		for (let i = 0; i < 8; i++) {
			advanceTime(db, 10);
			workerTick(db);
		}

		carrier = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === hunter._id,
		)!;
		expect(carrier.carrying).toBeNull();
		expect(
			eventMessages(db).some((message) => message.includes("to the shrine")),
		).toBe(true);
	});

	it("force-credits a straggler once the grace window lapses", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { food: 50, water: 100 });
		const straggler = getAliveCatsForTest(db, colony._id)[0];

		db.update(cats)
			.set({
				position: { map: "world", x: 60, y: 60 },
				carrying: { kind: "food", amount: 10, jobEndedAt: Date.now() - 61_000 },
			})
			.where(eq(cats._id, straggler._id))
			.run();

		const foodBefore = ensureGlobalColony(db).resources.food;
		advanceTime(db, 2);
		workerTick(db);

		const after = ensureGlobalColony(db).resources.food;
		expect(after).toBeGreaterThan(foodBefore + 9 - 1); // +10 minus tick consumption

		const carrier = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === straggler._id,
		)!;
		expect(carrier.carrying).toBeNull();

		// No double credit on the next tick.
		advanceTime(db, 2);
		workerTick(db);
		const depositEvents = (getGlobalDashboard(db)?.events ?? []).filter(
			(event: { type: string }) => event.type === "shrine_deposit",
		);
		expect(depositEvents).toHaveLength(1);
	});
});

describe("visible construction", () => {
	function insertConstructJob(
		colonyId: string,
		architectId: string,
		opts: { due?: boolean } = {},
	) {
		db.insert(jobs)
			.values({
				_id: "construct-visible-job",
				colonyId,
				kind: "build_house",
				status: "queued",
				requestedByType: "leader",
				requestedByPlayerId: null,
				assignedCatId: architectId,
				baseDurationSec: 600,
				speedMultiplier: 1,
				yieldMultiplier: 1,
				clickTimeReducedSec: 0,
				createdAt: Date.now(),
				startedAt: Date.now(),
				endsAt: opts.due ? Date.now() - 1000 : Date.now() + 600_000,
				metadata: { phase: "construct_house" },
			})
			.run();
	}

	it("breaks ground on a free site when construction starts", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { water: 100, materials: 100 });
		const architect = getAliveCatsForTest(db, colony._id)[0];
		insertConstructJob(colony._id, architect._id);

		advanceTime(db, 2);
		workerTick(db);

		const dashboard = getGlobalDashboard(db)!;
		const scaffolds = dashboard.buildings.filter(
			(b: { constructionProgress: number }) => b.constructionProgress < 100,
		);
		expect(scaffolds).toHaveLength(1);
		expect(scaffolds[0].type).toBe("den");

		// The job remembers its scaffold, and the architect heads there.
		const job = db
			.select()
			.from(jobs)
			.where(eq(jobs._id, "construct-visible-job"))
			.get()!;
		expect((job.metadata as { buildingId?: string }).buildingId).toBe(
			scaffolds[0]._id,
		);

		const worker = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === architect._id,
		)!;
		expect(worker.activity).toBe("traveling");
		expect(worker.destination).toEqual({
			map: "world",
			x: scaffolds[0].worldPosition.x,
			y: scaffolds[0].worldPosition.y,
		});
	});

	it("finishes the den when the job completes with enough resources", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { water: 100, materials: 100 });
		const architect = getAliveCatsForTest(db, colony._id)[0];
		insertConstructJob(colony._id, architect._id);

		advanceTime(db, 2);
		workerTick(db); // breaks ground
		forceJobDue(db, "construct-visible-job");
		advanceTime(db, 2);
		workerTick(db); // completes

		const dashboard = getGlobalDashboard(db)!;
		const dens = dashboard.buildings.filter(
			(b: { type: string }) => b.type === "den",
		);
		expect(dens).toHaveLength(6); // 5 starter + 1 new
		for (const den of dens) {
			expect(den.constructionProgress).toBe(100);
		}
		expect(eventMessages(db)).toContain(
			`${architect.name} finished building a new den.`,
		);
	});

	it("abandons the scaffold when resources run short at completion", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { water: 0, materials: 0 });
		const architect = getAliveCatsForTest(db, colony._id)[0];
		insertConstructJob(colony._id, architect._id);

		advanceTime(db, 2);
		workerTick(db); // breaks ground
		forceJobDue(db, "construct-visible-job");
		advanceTime(db, 2);
		workerTick(db); // completes without resources

		const dashboard = getGlobalDashboard(db)!;
		const dens = dashboard.buildings.filter(
			(b: { type: string }) => b.type === "den",
		);
		expect(dens).toHaveLength(5); // scaffold removed
	});

	it("reports housing pressure and village level on the dashboard", () => {
		ensureGlobalColony(db);
		const dashboard = getGlobalDashboard(db)!;

		// 20 cats vs shrine(4) + 5 dens (2 each) = 14 shelter
		expect(dashboard.housing.population).toBe(20);
		expect(dashboard.housing.capacity).toBe(14);
		expect(dashboard.housing.pressure).toBeCloseTo(20 / 14, 6);
		expect(dashboard.housing.villageLevel).toBe(2);
	});

	it("leader plans housing under crowding pressure", () => {
		ensureGlobalColony(db);
		setTestRngSeed(db, 3);

		// Pressure starts at 20/14 — the leader should react within a few
		// ticks once a policy roll passes.
		let planned = false;
		for (let i = 0; i < 10 && !planned; i++) {
			advanceTime(db, 5);
			workerTick(db);
			planned = getGlobalDashboard(db)!.jobs.some(
				(job: { kind: string }) => job.kind === "leader_plan_house",
			);
		}
		expect(planned).toBe(true);
	});
});

describe("elections", () => {
	function tick(seconds = 2) {
		advanceTime(db, seconds);
		workerTick(db);
	}

	function forceElectionDue(electionId: string) {
		db.update(elections)
			.set({ endsAt: Date.now() - 1000 })
			.where(eq(elections._id, electionId))
			.run();
	}

	it("opens a leadership election on the first tick", () => {
		ensureGlobalColony(db);
		tick();

		const dashboard = getGlobalDashboard(db)!;
		expect(dashboard.election).not.toBeNull();
		expect(dashboard.election!.candidates.length).toBeGreaterThan(0);
		expect(dashboard.election!.candidates.length).toBeLessThanOrEqual(5);
		expect(eventMessages(db)).toContain(
			"The colony is holding a leadership election — cast your vote!",
		);
	});

	it("elects the voted winner and stops auto-replacing the leader", () => {
		ensureGlobalColony(db);
		tick();

		const dashboard = getGlobalDashboard(db)!;
		const election = dashboard.election!;
		// Vote for the least leaderly candidate — votes must beat stats.
		const underdog = election.candidates[election.candidates.length - 1];

		castVote(db, {
			sessionId: "voter_1",
			nickname: "V1",
			electionId: election._id,
			catId: underdog._id,
		});
		castVote(db, {
			sessionId: "voter_2",
			nickname: "V2",
			electionId: election._id,
			catId: underdog._id,
		});

		forceElectionDue(election._id);
		tick();

		const colony = ensureGlobalColony(db);
		expect(colony.leaderId).toBe(underdog._id);

		// The underdog stays in charge — no per-tick auto-replace.
		tick(30);
		expect(ensureGlobalColony(db).leaderId).toBe(underdog._id);
	});

	it("counts a changed vote once", () => {
		ensureGlobalColony(db);
		tick();
		const election = getGlobalDashboard(db)!.election!;
		const [first, , third] = election.candidates;

		castVote(db, {
			sessionId: "swing_voter",
			nickname: "SV",
			electionId: election._id,
			catId: third._id,
		});
		castVote(db, {
			sessionId: "swing_voter",
			nickname: "SV",
			electionId: election._id,
			catId: first._id,
		});

		const tally = getGlobalDashboard(db)!.election!.tally;
		expect(tally[first._id]).toBe(1);
		expect(tally[third._id]).toBeUndefined();
	});

	it("kicks the leader with 5 distinct signatures and bars them from the snap election", () => {
		ensureGlobalColony(db);
		tick();

		// Close the bootstrap election so the snap election is observable.
		const bootstrapElection = getGlobalDashboard(db)!.election!;
		forceElectionDue(bootstrapElection._id);
		tick();

		const leaderId = ensureGlobalColony(db).leaderId!;

		requestVoteKick(db, { sessionId: "angry_1", nickname: "A1" });
		const petition = getGlobalDashboard(db)!.voteKick!;
		expect(petition.targetCatId).toBe(leaderId);
		expect(petition.signatures).toBe(1);

		for (let i = 2; i <= 5; i++) {
			castVote(db, {
				sessionId: `angry_${i}`,
				nickname: `A${i}`,
				electionId: petition._id,
				catId: leaderId,
			});
		}
		expect(getGlobalDashboard(db)!.voteKick!.signatures).toBe(5);

		forceElectionDue(petition._id);
		tick();

		const colony = ensureGlobalColony(db);
		expect(colony.leaderId).not.toBe(leaderId);
		expect(eventMessages(db).some((m) => m.includes("was voted out"))).toBe(
			true,
		);

		// Snap election opened, kicked cat is not on the ballot.
		const snap = getGlobalDashboard(db)!.election!;
		expect(snap.candidates.map((c: { _id: string }) => c._id)).not.toContain(
			leaderId,
		);
	});

	it("leaves the leader in place with fewer than 5 signatures", () => {
		ensureGlobalColony(db);
		tick();
		const leaderId = ensureGlobalColony(db).leaderId!;

		requestVoteKick(db, { sessionId: "grump_1", nickname: "G1" });
		const petition = getGlobalDashboard(db)!.voteKick!;
		for (let i = 2; i <= 4; i++) {
			castVote(db, {
				sessionId: `grump_${i}`,
				nickname: `G${i}`,
				electionId: petition._id,
				catId: leaderId,
			});
		}

		forceElectionDue(petition._id);
		tick();

		expect(ensureGlobalColony(db).leaderId).toBe(leaderId);
	});
});

describe("player zones", () => {
	const SESSION_Z = { sessionId: "zoner_1", nickname: "Zoner" };

	it("creates, lists, and removes a zone", () => {
		ensureGlobalColony(db);
		const { zoneId } = createZone(db, {
			...SESSION_Z,
			kind: "gather",
			a: { x: 10, y: 10 },
			b: { x: 14, y: 12 },
			durationMs: 30 * 60 * 1000,
		}) as { zoneId: string };

		let dashboard = getGlobalDashboard(db)!;
		expect(dashboard.zones).toHaveLength(1);
		expect(dashboard.zones[0]).toMatchObject({
			kind: "gather",
			x1: 10,
			y1: 10,
			x2: 14,
			y2: 12,
		});

		removeZone(db, { ...SESSION_Z, zoneId });
		dashboard = getGlobalDashboard(db)!;
		expect(dashboard.zones).toHaveLength(0);
	});

	it("enforces per-player limits, size, duration, and ownership", () => {
		ensureGlobalColony(db);
		const make = (x: number) =>
			createZone(db, {
				...SESSION_Z,
				kind: "avoid",
				a: { x, y: 0 },
				b: { x: x + 2, y: 2 },
				durationMs: 30 * 60 * 1000,
			}) as { zoneId: string };

		const first = make(0);
		make(10);
		expect(() => make(20)).toThrow(/active zones/);

		expect(() =>
			createZone(db, {
				sessionId: "other",
				nickname: "O",
				kind: "avoid",
				a: { x: 0, y: 0 },
				b: { x: 8, y: 2 }, // 9 tiles wide
				durationMs: 30 * 60 * 1000,
			}),
		).toThrow(/limited/);

		expect(() =>
			createZone(db, {
				sessionId: "other",
				nickname: "O",
				kind: "avoid",
				a: { x: 0, y: 0 },
				b: { x: 2, y: 2 },
				durationMs: 1000,
			}),
		).toThrow(/duration/);

		expect(() =>
			removeZone(db, {
				sessionId: "other",
				nickname: "O",
				zoneId: first.zoneId,
			}),
		).toThrow(/your own/);
	});

	it("sweeps expired zones during the tick", () => {
		ensureGlobalColony(db);
		createZone(db, {
			...SESSION_Z,
			kind: "avoid",
			a: { x: 0, y: 0 },
			b: { x: 3, y: 3 },
			durationMs: 10 * 60 * 1000,
		});

		// Not expired yet
		advanceTime(db, 2);
		workerTick(db);
		expect(getGlobalDashboard(db)!.zones).toHaveLength(1);

		// Push past expiry (advanceTime shifts lastTick, not wall time — so
		// force the zone's expiry into the past instead).
		const zone = getGlobalDashboard(db)!.zones[0];
		db.update(zonesTable)
			.set({ expiresAt: Date.now() - 1000 })
			.where(eq(zonesTable._id, zone._id))
			.run();
		advanceTime(db, 2);
		workerTick(db);
		expect(getGlobalDashboard(db)!.zones).toHaveLength(0);
	});

	it("keeps wandering cats out of avoid zones", () => {
		ensureGlobalColony(db);
		setTestRngSeed(db, 1);

		// Blanket the whole wander area (anchor 6,6 ± 3) with an avoid zone
		// from two players (zones are max 8x8).
		createZone(db, {
			...SESSION_Z,
			kind: "avoid",
			a: { x: 3, y: 3 },
			b: { x: 9, y: 9 },
			durationMs: 30 * 60 * 1000,
		});

		for (let i = 0; i < 5; i++) {
			advanceTime(db, 60);
			workerTick(db);
		}

		const colony = ensureGlobalColony(db);
		const wanderers = getAliveCatsForTest(db, colony._id).filter(
			(cat) => cat.destination !== null,
		);
		// No cat may target the blanketed clearing.
		for (const cat of wanderers) {
			const dest = cat.destination!;
			const inAvoid = dest.x >= 3 && dest.x <= 9 && dest.y >= 3 && dest.y <= 9;
			expect(inAvoid).toBe(false);
		}
	});
});

describe("production (Phase 7)", () => {
	const SESSION_P = { sessionId: "producer_1", nickname: "Producer" };

	function insertWorkshop(colonyId: string, id = "workshop-1") {
		db.insert(buildingsTable)
			.values({
				_id: id,
				colonyId,
				type: "workshop",
				level: 1,
				position: { x: 2, y: 2 },
				constructionProgress: 100,
				productionProgress: 0,
			})
			.run();
		return id;
	}

	it("gates building types by village level", () => {
		ensureGlobalColony(db);
		// Starter village is level 2: workshops yes, fields not yet.
		const result = planBuilding(db, { ...SESSION_P, type: "workshop" }) as {
			ok: boolean;
			jobId?: string;
		};
		expect(result.ok).toBe(true);

		expect(() => planBuilding(db, { ...SESSION_P, type: "field" })).toThrow(
			/village level 4/,
		);

		// Duplicate pending builds are rejected softly.
		const dupe = planBuilding(db, { ...SESSION_P, type: "workshop" }) as {
			ok: boolean;
			reason?: string;
		};
		expect(dupe.ok).toBe(false);
		expect(dupe.reason).toBe("already_in_progress");
	});

	it("builds a workshop scaffold of the right type", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { water: 100, materials: 100 });
		planBuilding(db, { ...SESSION_P, type: "workshop" });

		advanceTime(db, 2);
		workerTick(db); // promotes and breaks ground

		const scaffold = getGlobalDashboard(db)!.buildings.find(
			(b: { type: string }) => b.type === "workshop",
		);
		expect(scaffold).toBeTruthy();
		expect(scaffold?.constructionProgress).toBeLessThan(100);
	});

	it("refines materials with an assigned worker and stalls without", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { food: 200, water: 200, materials: 50 });
		const shopId = insertWorkshop(colony._id);
		const worker = getAliveCatsForTest(db, colony._id)[0];

		assignWorker(db, { ...SESSION_P, catId: worker._id, buildingId: shopId });

		// A full cycle is 600s of worker time.
		advanceTime(db, 700);
		workerTick(db);

		const colonyAfter = ensureGlobalColony(db);
		expect(colonyAfter.resources.refined ?? 0).toBeGreaterThanOrEqual(1);
		expect(colonyAfter.resources.materials).toBeLessThanOrEqual(45);

		// Unassigning frees the cat, but the leader auto-staffs workerless
		// workshops — production continues under new management.
		assignWorker(db, { ...SESSION_P, catId: worker._id, buildingId: null });
		advanceTime(db, 700);
		workerTick(db);
		expect(
			eventMessages(db).some((message) =>
				message.includes("to work at the workshop"),
			),
		).toBe(true);
	});

	it("validates worker assignment targets", () => {
		const colony = ensureGlobalColony(db);
		const cat = getAliveCatsForTest(db, colony._id)[0];
		expect(() =>
			assignWorker(db, { ...SESSION_P, catId: cat._id, buildingId: "nope" }),
		).toThrow(/cannot take a worker/);
		expect(() =>
			assignWorker(db, {
				...SESSION_P,
				catId: "ghost-cat",
				buildingId: null,
			}),
		).toThrow(/not available/);
	});
});

describe("cat movement", () => {
	it("sends the assigned hunter out when a hunt is promoted to active", () => {
		const colony = ensureGlobalColony(db);
		setTestRngSeed(db, 7);
		const hunter = getAliveCatsForTest(db, colony._id)[0];

		db.insert(jobs)
			.values({
				_id: "hunt-travel-job",
				colonyId: colony._id,
				kind: "hunt_expedition",
				status: "queued",
				requestedByType: "leader",
				requestedByPlayerId: null,
				assignedCatId: hunter._id,
				baseDurationSec: 8 * 3600,
				speedMultiplier: 1,
				yieldMultiplier: 1,
				clickTimeReducedSec: 0,
				createdAt: Date.now(),
				startedAt: Date.now(),
				endsAt: Date.now() + 8 * 3600 * 1000,
				metadata: {},
			})
			.run();

		advanceTime(db, 2);
		workerTick(db);

		const updated = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === hunter._id,
		)!;
		expect(updated.activity).toBe("traveling");
		expect(updated.destination).not.toBeNull();
		const dest = updated.destination!;
		expect(dest.map).toBe("world");
		// Hunts happen outside the village clearing.
		expect(
			Math.max(Math.abs(dest.x - 6), Math.abs(dest.y - 6)),
		).toBeGreaterThan(4);
	});

	it("walks a traveling cat toward its destination and sets it to work on arrival", () => {
		const colony = ensureGlobalColony(db);
		const walker = getAliveCatsForTest(db, colony._id)[0];

		db.update(cats)
			.set({
				position: { map: "world", x: 6, y: 6 },
				destination: { map: "world", x: 16, y: 6 },
				activity: "traveling",
			})
			.where(eq(cats._id, walker._id))
			.run();

		advanceTime(db, 10);
		workerTick(db);

		let updated = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === walker._id,
		)!;
		// 10s at 0.5 tiles/s — halfway there, still traveling.
		expect(updated.position.x).toBeCloseTo(11, 5);
		expect(updated.position.y).toBe(6);
		expect(updated.activity).toBe("traveling");

		advanceTime(db, 60);
		workerTick(db);

		updated = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === walker._id,
		)!;
		expect(updated.position.x).toBe(16);
		expect(updated.activity).toBe("working");
		expect(updated.destination).toBeNull();
	});

	it("returns a cat home and settles it back to idle", () => {
		const colony = ensureGlobalColony(db);
		const traveler = getAliveCatsForTest(db, colony._id)[0];

		db.update(cats)
			.set({
				position: { map: "world", x: 20, y: 6 },
				destination: { map: "world", x: 7, y: 7 },
				activity: "returning",
			})
			.where(eq(cats._id, traveler._id))
			.run();

		advanceTime(db, 120);
		workerTick(db);

		const updated = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === traveler._id,
		)!;
		expect(updated.position).toEqual({ map: "world", x: 7, y: 7 });
		expect(updated.activity).toBe("idle");
		expect(updated.destination).toBeNull();
	});

	it("sends the worker home when its job completes", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { food: 100, water: 100 });
		const ritualist = getAliveCatsForTest(db, colony._id)[0];

		// Far from the village so the same-tick movement pass can't already
		// bring it home (which would legitimately flip it back to idle).
		db.update(cats)
			.set({ position: { map: "world", x: 40, y: 6 } })
			.where(eq(cats._id, ritualist._id))
			.run();

		db.insert(jobs)
			.values({
				_id: "ritual-return-job",
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

		advanceTime(db, 2);
		workerTick(db);

		const updated = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === ritualist._id,
		)!;
		expect(updated.activity).toBe("returning");
		expect(updated.destination).not.toBeNull();
		const home = updated.destination!;
		// Home spots are inside the village clearing.
		expect(
			Math.max(Math.abs(home.x - 6), Math.abs(home.y - 6)),
		).toBeLessThanOrEqual(3);
	});

	it("lets idle cats pick wander destinations under a seeded RNG", () => {
		ensureGlobalColony(db);
		setTestRngSeed(db, 1);

		advanceTime(db, 60);
		workerTick(db);

		const wanderers = getAliveCatsForTest(
			db,
			ensureGlobalColony(db)._id,
		).filter((cat) => cat.destination !== null);
		expect(wanderers.length).toBeGreaterThan(0);
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
