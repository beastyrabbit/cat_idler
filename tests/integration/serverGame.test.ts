/**
 * Integration tests for server/game.ts against an in-memory SQLite DB.
 *
 * These cover the simulation entry points that were previously only
 * exercisable through a running Convex deployment: bootstrap, jobs,
 * upgrades, click boosting, the worker tick, and run resets.
 */

import { and, eq, isNull } from "drizzle-orm";
import { nanoid } from "nanoid";
import { beforeEach, describe, expect, it } from "vitest";
import { createDb, type GameDb } from "@/db/client";
import {
	buildings as buildingsTable,
	cats,
	colonies,
	elections,
	events,
	jobs,
	runHistory,
	worldTiles,
	zones as zonesTable,
} from "@/db/schema";
import { getLifeStage } from "@/lib/game/lifeSim";
import {
	colonyToWorld,
	ringCells,
	VILLAGE_ANCHOR,
} from "@/lib/game/villageLayout";
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

describe("world depletion & tree chopping", () => {
	it("drains the hunt site tile's food by the hauled share on a mid-job trip", () => {
		const colony = ensureGlobalColony(db);
		setTestRngSeed(db, 7);
		const hunter = getAliveCatsForTest(db, colony._id)[0];

		const site = { x: 10, y: 6 };
		// Give the site tile a known, plentiful food stock as a plain field.
		db.update(worldTiles)
			.set({
				type: "field",
				resources: { food: 50, herbs: 0, water: 0 },
				maxResources: { food: 60, herbs: 0 },
				lastDepleted: 0,
			})
			.where(
				and(
					eq(worldTiles.colonyId, colony._id),
					eq(worldTiles.x, site.x),
					eq(worldTiles.y, site.y),
				),
			)
			.run();

		const now = Date.now();
		db.insert(jobs)
			.values({
				_id: "hunt-drain-job",
				colonyId: colony._id,
				kind: "hunt_expedition",
				status: "active",
				requestedByType: "leader",
				requestedByPlayerId: null,
				assignedCatId: hunter._id,
				baseDurationSec: 8 * 3600,
				speedMultiplier: 1,
				yieldMultiplier: 1,
				clickTimeReducedSec: 0,
				createdAt: now,
				startedAt: now - 1000,
				endsAt: now + 8 * 3600 * 1000,
				metadata: {
					site,
					accepted: true,
					tripsDone: 0,
					totalYield: 30,
					nextTripAt: now - 1000,
				},
			})
			.run();

		// The hunter is standing at the site, ready to haul a share home.
		db.update(cats)
			.set({
				activity: "working",
				carrying: null,
				position: { map: "world", ...site },
			})
			.where(eq(cats._id, hunter._id))
			.run();

		advanceTime(db, 2);
		workerTick(db);

		const tile = db
			.select()
			.from(worldTiles)
			.where(
				and(
					eq(worldTiles.colonyId, colony._id),
					eq(worldTiles.x, site.x),
					eq(worldTiles.y, site.y),
				),
			)
			.get()!;
		// splitYield(30, 3, 0) === 10 hauled off the site this trip.
		expect(tile.resources.food).toBe(40);
		expect(tile.lastDepleted).toBeGreaterThan(0);
	});

	it("chops the nearest explored forest into a permanent field on gather_materials completion", () => {
		const colony = ensureGlobalColony(db);
		const builder = getAliveCatsForTest(db, colony._id)[0];

		// Flatten the map to non-forest, then plant one explored forest tile so
		// the chop target is deterministic.
		db.update(worldTiles)
			.set({ type: "field" })
			.where(eq(worldTiles.colonyId, colony._id))
			.run();
		const forest = { x: 6, y: 10 }; // Chebyshev 4 from the (6,6) anchor -> explored
		db.update(worldTiles)
			.set({
				type: "forest",
				resources: { food: 40, herbs: 5, water: 0 },
				maxResources: { food: 60, herbs: 0 },
				pathWear: 0,
				lastDepleted: 0,
			})
			.where(
				and(
					eq(worldTiles.colonyId, colony._id),
					eq(worldTiles.x, forest.x),
					eq(worldTiles.y, forest.y),
				),
			)
			.run();

		const now = Date.now();
		db.insert(jobs)
			.values({
				_id: "gather-chop-job",
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
				createdAt: now,
				startedAt: now - 1000,
				endsAt: now - 1000,
				metadata: { phase: "gather_materials" },
			})
			.run();

		advanceTime(db, 2);
		workerTick(db);

		const chopped = db
			.select()
			.from(worldTiles)
			.where(
				and(
					eq(worldTiles.colonyId, colony._id),
					eq(worldTiles.x, forest.x),
					eq(worldTiles.y, forest.y),
				),
			)
			.get()!;
		expect(chopped.type).toBe("field");
		expect(chopped.resources.food).toBe(0);
		expect(chopped.resources.herbs).toBe(0);
		expect(chopped.maxResources.food).toBe(5);
		expect(chopped.lastDepleted).toBeGreaterThan(0);

		expect(eventMessages(db)).toContain(
			`${builder.name} chopped the forest at (6, 10) for lumber.`,
		);
	});

	it("regrows a depleted field up to its cap and never resurrects a chopped forest's type", () => {
		const colony = ensureGlobalColony(db);

		// A chopped-forest remnant: permanently a field with a low food cap.
		const spot = { x: 8, y: 8 };
		db.update(worldTiles)
			.set({
				type: "field",
				resources: { food: 0, herbs: 0, water: 0 },
				maxResources: { food: 5, herbs: 0 },
				lastDepleted: Date.now(),
			})
			.where(
				and(
					eq(worldTiles.colonyId, colony._id),
					eq(worldTiles.x, spot.x),
					eq(worldTiles.y, spot.y),
				),
			)
			.run();

		const read = () =>
			db
				.select()
				.from(worldTiles)
				.where(
					and(
						eq(worldTiles.colonyId, colony._id),
						eq(worldTiles.x, spot.x),
						eq(worldTiles.y, spot.y),
					),
				)
				.get()!;

		// One game-hour of regrowth: +1 food/hr at timeScale 1.
		advanceTime(db, 3600);
		workerTick(db);
		let tile = read();
		expect(tile.type).toBe("field");
		expect(tile.resources.food).toBeGreaterThan(0);
		expect(tile.resources.food).toBeLessThan(5);

		// Several more hours can't push past the cap.
		advanceTime(db, 6 * 3600);
		workerTick(db);
		tile = read();
		expect(tile.resources.food).toBe(5);
		expect(tile.type).toBe("field");
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

	it("hunters haul the catch home in trips and return to their site", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { food: 50, water: 200 });
		const hunter = getAliveCatsForTest(db, colony._id)[0];

		// Cat is at its hunt site, mid-job, with the first trip overdue.
		db.update(cats)
			.set({
				position: { map: "world", x: 6, y: 20 },
				activity: "working",
				currentTask: "hunt_expedition",
			})
			.where(eq(cats._id, hunter._id))
			.run();
		db.insert(jobs)
			.values({
				_id: "hunt-trip-job",
				colonyId: colony._id,
				kind: "hunt_expedition",
				status: "active",
				requestedByType: "leader",
				requestedByPlayerId: null,
				assignedCatId: hunter._id,
				baseDurationSec: 8 * 3600,
				speedMultiplier: 1,
				yieldMultiplier: 1,
				clickTimeReducedSec: 0,
				createdAt: Date.now() - 4 * 3600 * 1000,
				startedAt: Date.now() - 4 * 3600 * 1000,
				endsAt: Date.now() + 4 * 3600 * 1000,
				metadata: {
					accepted: true,
					site: { x: 6, y: 20 },
					tripsDone: 0,
					nextTripAt: Date.now() - 1000,
				},
			})
			.run();

		advanceTime(db, 2);
		workerTick(db);

		// Departed for the shrine with roughly a third of the catch.
		let updated = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === hunter._id,
		)!;
		expect(updated.carrying).not.toBeNull();
		expect(updated.activity).toBe("returning");
		const meta = db
			.select()
			.from(jobs)
			.where(eq(jobs._id, "hunt-trip-job"))
			.get()!.metadata as { tripsDone?: number; totalYield?: number };
		expect(meta.tripsDone).toBe(1);
		expect(updated.carrying!.amount).toBeLessThan(meta.totalYield ?? 0);

		// Walk home, deposit, and head straight back to the site.
		let backOut = false;
		for (let i = 0; i < 20 && !backOut; i++) {
			advanceTime(db, 60);
			workerTick(db);
			updated = getAliveCatsForTest(db, colony._id).find(
				(cat) => cat._id === hunter._id,
			)!;
			backOut =
				updated.carrying === null &&
				updated.destination?.x === 6 &&
				updated.destination?.y === 20;
		}
		expect(backOut).toBe(true);
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

describe("quarry expeditions", () => {
	it("hauls materials off a quarry site and banks them at the shrine", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { food: 100, water: 200, materials: 0 });
		const miner = getAliveCatsForTest(db, colony._id)[0];

		const site = { x: 6, y: 20 };
		db.update(cats)
			.set({
				position: { map: "world", ...site },
				activity: "working",
				currentTask: "quarry",
				carrying: null,
			})
			.where(eq(cats._id, miner._id))
			.run();

		const now = Date.now();
		db.insert(jobs)
			.values({
				_id: "quarry-haul-job",
				colonyId: colony._id,
				kind: "quarry",
				status: "active",
				requestedByType: "leader",
				requestedByPlayerId: null,
				assignedCatId: miner._id,
				baseDurationSec: 2 * 3600,
				speedMultiplier: 1,
				yieldMultiplier: 1,
				clickTimeReducedSec: 0,
				createdAt: now - 3600 * 1000,
				startedAt: now - 3600 * 1000,
				endsAt: now + 3600 * 1000,
				metadata: {
					accepted: true,
					site,
					tripsDone: 0,
					totalYield: 15,
					nextTripAt: now - 1000,
				},
			})
			.run();

		const materialsBefore = ensureGlobalColony(db).resources.materials;
		advanceTime(db, 2);
		workerTick(db);

		// A share of the load is on the miner's back, heading for the shrine.
		let carrier = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === miner._id,
		)!;
		expect(carrier.carrying).not.toBeNull();
		expect(carrier.carrying!.kind).toBe("materials");
		// splitYield(15, 3, 0) === 5 hauled this trip.
		expect(carrier.carrying!.amount).toBe(5);
		expect(carrier.activity).toBe("returning");

		// Walk home (14 tiles at 0.5/s) and deposit at the shrine.
		for (let i = 0; i < 10; i++) {
			advanceTime(db, 10);
			workerTick(db);
		}

		carrier = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === miner._id,
		)!;
		expect(carrier.carrying).toBeNull();
		expect(ensureGlobalColony(db).resources.materials).toBe(
			materialsBefore + 5,
		);
		expect(
			eventMessages(db).some((message) =>
				message.includes("materials to the shrine"),
			),
		).toBe(true);
	});
});

describe("explore expeditions", () => {
	it("sends a scout to an unexplored frontier tile", () => {
		const colony = ensureGlobalColony(db);
		// Clear worn paths so only tiles within village sight (Chebyshev 6)
		// count as explored — the frontier is then the ring just beyond.
		db.update(worldTiles)
			.set({ pathWear: 0 })
			.where(eq(worldTiles.colonyId, colony._id))
			.run();
		const scout = getAliveCatsForTest(db, colony._id)[0];

		db.insert(jobs)
			.values({
				_id: "explore-dispatch-job",
				colonyId: colony._id,
				kind: "explore",
				status: "queued",
				requestedByType: "leader",
				requestedByPlayerId: null,
				assignedCatId: scout._id,
				baseDurationSec: 30 * 60,
				speedMultiplier: 1,
				yieldMultiplier: 1,
				clickTimeReducedSec: 0,
				createdAt: Date.now(),
				startedAt: Date.now(),
				endsAt: Date.now() + 30 * 60 * 1000,
				metadata: {},
			})
			.run();

		advanceTime(db, 2);
		workerTick(db); // promotes queued -> active, assigns a frontier site

		const promoted = db
			.select()
			.from(jobs)
			.where(eq(jobs._id, "explore-dispatch-job"))
			.get()!;
		const site = (promoted.metadata as { site?: { x: number; y: number } })
			.site;
		expect(site).toBeDefined();
		// Nearest frontier is the fog ring one step past village sight.
		const cheb = Math.max(Math.abs(site!.x - 6), Math.abs(site!.y - 6));
		expect(cheb).toBe(7);

		const tile = db
			.select()
			.from(worldTiles)
			.where(
				and(
					eq(worldTiles.colonyId, colony._id),
					eq(worldTiles.x, site!.x),
					eq(worldTiles.y, site!.y),
				),
			)
			.get()!;
		expect(tile.pathWear).toBeLessThanOrEqual(62);

		const traveler = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === scout._id,
		)!;
		expect(traveler.activity).toBe("traveling");
		expect(traveler.currentTask).toBe("explore");
	});

	it("logs a mapped-the-lands event when a scout finishes", () => {
		const colony = ensureGlobalColony(db);
		const scout = getAliveCatsForTest(db, colony._id)[0];

		const now = Date.now();
		db.insert(jobs)
			.values({
				_id: "explore-complete-job",
				colonyId: colony._id,
				kind: "explore",
				status: "active",
				requestedByType: "leader",
				requestedByPlayerId: null,
				assignedCatId: scout._id,
				baseDurationSec: 30 * 60,
				speedMultiplier: 1,
				yieldMultiplier: 1,
				clickTimeReducedSec: 0,
				createdAt: now - 60_000,
				startedAt: now - 60_000,
				endsAt: now - 1000,
				metadata: { accepted: true, site: { x: 13, y: 6 } },
			})
			.run();

		advanceTime(db, 2);
		workerTick(db);

		expect(
			eventMessages(db).some((message) =>
				message.includes("mapped the lands around (13, 6)"),
			),
		).toBe(true);
	});
});

describe("travel trail integrity", () => {
	it("wears every tile of a long L-route in one accelerated tick", () => {
		const colony = ensureGlobalColony(db);
		setResources(db, colony._id, { food: 100, water: 100 });
		const traveler = getAliveCatsForTest(db, colony._id)[0];

		// Start well outside the village fence and head to a far corner that
		// forces both an x-leg and a y-leg (an L). Clear any prior wear so the
		// only thing that can raise pathWear is the cat physically walking.
		const start = { x: 12, y: 6 };
		const dest = { x: 20, y: 14 };
		db.update(worldTiles)
			.set({ pathWear: 0 })
			.where(eq(worldTiles.colonyId, colony._id))
			.run();
		db.update(cats)
			.set({
				position: { map: "world", ...start },
				destination: { map: "world", ...dest },
				activity: "traveling",
				currentTask: null,
				carrying: null,
			})
			.where(eq(cats._id, traveler._id))
			.run();

		// A huge movement budget in a single tick: pre-fix this teleports the
		// cat along one axis only, leaving the rest of the route untrodden.
		db.update(colonies)
			.set({ testTimeScale: 500 })
			.where(eq(colonies._id, colony._id))
			.run();
		advanceTime(db, 5);
		workerTick(db);

		const wearAt = (x: number, y: number) =>
			db
				.select()
				.from(worldTiles)
				.where(
					and(
						eq(worldTiles.colonyId, colony._id),
						eq(worldTiles.x, x),
						eq(worldTiles.y, y),
					),
				)
				.get()!.pathWear;

		// Reveal threshold is >62; a trodden route tile lands at >=64. Sample
		// intermediate tiles on BOTH legs, not just the endpoints.
		const REVEAL = 62;
		for (const [x, y] of [
			[14, 6], // x-leg, mid
			[18, 6], // x-leg, near corner
			[20, 6], // the corner
			[20, 9], // y-leg, mid
			[20, 12], // y-leg, near end
			[20, 14], // destination
		] as const) {
			expect(wearAt(x, y)).toBeGreaterThan(REVEAL);
		}

		// The whole journey happened this tick, so the cat is at the far corner.
		const walked = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === traveler._id,
		)!;
		expect(walked.position).toMatchObject({ map: "world", ...dest });
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

		// The architect reports in at the shrine before heading to the site.
		const worker = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === architect._id,
		)!;
		expect(worker.activity).toBe("traveling");
		expect(worker.destination).toEqual({ map: "world", x: 6, y: 6 });
		expect(
			(
				db
					.select()
					.from(jobs)
					.where(eq(jobs._id, "construct-visible-job"))
					.get()!.metadata as { site?: { x: number; y: number } }
			).site,
		).toEqual({
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
		// workshops — production continues under new management. Staffing is
		// gated by an (unseeded) policy-reliability roll, so allow the
		// leader a few ticks to get around to it.
		assignWorker(db, { ...SESSION_P, catId: worker._id, buildingId: null });
		let staffed = false;
		for (let i = 0; i < 12 && !staffed; i++) {
			advanceTime(db, 700);
			workerTick(db);
			staffed = eventMessages(db).some((message) =>
				message.includes("to work at the workshop"),
			);
		}
		expect(staffed).toBe(true);
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

		// Jobs are accepted at the shrine: the cat reports in first while
		// the job records the hunt site out beyond the fence.
		const updated = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === hunter._id,
		)!;
		expect(updated.activity).toBe("traveling");
		expect(updated.destination).toEqual({ map: "world", x: 6, y: 6 });

		const meta = db
			.select()
			.from(jobs)
			.where(eq(jobs._id, "hunt-travel-job"))
			.get()!.metadata as { site?: { x: number; y: number } };
		expect(meta.site).toBeTruthy();
		expect(
			Math.max(Math.abs(meta.site!.x - 6), Math.abs(meta.site!.y - 6)),
		).toBeGreaterThan(4);

		let accepted = false;
		for (let i = 0; i < 10 && !accepted; i++) {
			advanceTime(db, 30);
			workerTick(db);
			accepted =
				(
					db.select().from(jobs).where(eq(jobs._id, "hunt-travel-job")).get()!
						.metadata as { accepted?: boolean }
				).accepted === true;
		}
		expect(accepted).toBe(true);
	});

	it("walks a traveling cat toward its destination and sets it to work on arrival", () => {
		const colony = ensureGlobalColony(db);
		const walker = getAliveCatsForTest(db, colony._id)[0];

		// Worldgen seeds trail wear that grants speed bonuses — flatten it
		// so the base walking speed is observable.
		db.update(worldTiles)
			.set({ pathWear: 0 })
			.where(eq(worldTiles.colonyId, colony._id))
			.run();

		// Entirely outside the village fence so no gate detour applies.
		db.update(cats)
			.set({
				position: { map: "world", x: 20, y: 6 },
				destination: { map: "world", x: 30, y: 6 },
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
		expect(updated.position.x).toBeCloseTo(25, 5);
		expect(updated.position.y).toBe(6);
		expect(updated.activity).toBe("traveling");

		advanceTime(db, 60);
		workerTick(db);

		updated = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === walker._id,
		)!;
		expect(updated.position.x).toBe(30);
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

		// Several legs: 4-directional hops through the south gate, then home.
		// Stop as soon as it lands (idle cats wander off again afterwards).
		let updated = getAliveCatsForTest(db, colony._id).find(
			(cat) => cat._id === traveler._id,
		)!;
		for (let i = 0; i < 8; i++) {
			advanceTime(db, 120);
			workerTick(db);
			updated = getAliveCatsForTest(db, colony._id).find(
				(cat) => cat._id === traveler._id,
			)!;
			if (updated.position.x === 7 && updated.position.y === 7) {
				break;
			}
		}
		expect(updated.position).toEqual({ map: "world", x: 7, y: 7 });
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

describe("world credibility", () => {
	it("never raises a scaffold on a water tile", () => {
		const colony = ensureGlobalColony(db);
		const existing = db
			.select()
			.from(buildingsTable)
			.where(eq(buildingsTable.colonyId, colony._id))
			.all();
		const occupied = new Set(
			existing.map((b) => `${b.position.x},${b.position.y}`),
		);

		// Leave a single free build cell in rings 1-2 and make it water; every
		// other inner cell is filled, so the next scaffold must either land on
		// that water cell (the bug) or skip past it to ring 3 (the fix).
		const target = ringCells(2).find(
			(cell) => !occupied.has(`${cell.x},${cell.y}`),
		)!;
		for (const cell of [...ringCells(1), ...ringCells(2)]) {
			const key = `${cell.x},${cell.y}`;
			if (occupied.has(key) || (cell.x === target.x && cell.y === target.y)) {
				continue;
			}
			db.insert(buildingsTable)
				.values({
					_id: nanoid(),
					colonyId: colony._id,
					type: "den",
					level: 1,
					position: cell,
					constructionProgress: 100,
				})
				.run();
		}

		const targetWorld = colonyToWorld(target);
		db.update(worldTiles)
			.set({
				type: "river",
				overlayFeature: "river",
				resources: { food: 0, herbs: 0, water: 999 },
			})
			.where(
				and(
					eq(worldTiles.colonyId, colony._id),
					eq(worldTiles.x, targetWorld.x),
					eq(worldTiles.y, targetWorld.y),
				),
			)
			.run();

		// Queue a construction job; the worker places its scaffold on promotion.
		const architect = getAliveCatsForTest(db, colony._id)[0];
		const now = Date.now();
		db.insert(jobs)
			.values({
				_id: nanoid(),
				colonyId: colony._id,
				kind: "build_house",
				status: "queued",
				requestedByType: "leader",
				assignedCatId: architect._id,
				baseDurationSec: 1000,
				speedMultiplier: 1,
				yieldMultiplier: 1,
				clickTimeReducedSec: 0,
				createdAt: now,
				startedAt: now,
				endsAt: now + 1_000_000,
				metadata: { phase: "construct_house", buildingType: "food_storage" },
			})
			.run();

		advanceTime(db, 5);
		workerTick(db);

		const scaffold = db
			.select()
			.from(buildingsTable)
			.where(
				and(
					eq(buildingsTable.colonyId, colony._id),
					eq(buildingsTable.type, "food_storage"),
				),
			)
			.all()
			.find((b) => b.constructionProgress < 100);

		expect(scaffold).toBeDefined();
		// It skipped the only free inner cell because that cell is water.
		expect(scaffold?.position).not.toEqual(target);
		const world = colonyToWorld(scaffold!.position);
		const tile = db
			.select()
			.from(worldTiles)
			.where(
				and(
					eq(worldTiles.colonyId, colony._id),
					eq(worldTiles.x, world.x),
					eq(worldTiles.y, world.y),
				),
			)
			.get();
		expect(tile?.type).not.toBe("river");
		expect(tile?.resources.water ?? 0).toBe(0);
	});

	it("fetches its own water when the reservoir runs low", () => {
		const colony = ensureGlobalColony(db);
		// Drain the reservoir and guarantee a known, explored water tile.
		db.update(colonies)
			.set({ resources: { ...colony.resources, water: 15, food: 400 } })
			.where(eq(colonies._id, colony._id))
			.run();
		db.update(worldTiles)
			.set({
				type: "river",
				overlayFeature: "river",
				resources: { food: 0, herbs: 0, water: 999 },
			})
			.where(
				and(
					eq(worldTiles.colonyId, colony._id),
					eq(worldTiles.x, VILLAGE_ANCHOR.x),
					eq(worldTiles.y, VILLAGE_ANCHOR.y + 4),
				),
			)
			.run();

		setTestRngSeed(db, 7);
		setTestAcceleration(db, "hyper");

		for (let i = 0; i < 8; i++) {
			advanceTime(db, 60);
			workerTick(db);
		}

		const waterJobs = db
			.select()
			.from(jobs)
			.where(and(eq(jobs.colonyId, colony._id), eq(jobs.kind, "fetch_water")))
			.all();
		expect(waterJobs.length).toBeGreaterThan(0);
	});

	it("reveals a 3x3 fog halo around moving cats", () => {
		const colony = ensureGlobalColony(db);
		setTestRngSeed(db, 3);
		setTestAcceleration(db, "hyper");

		for (let i = 0; i < 20; i++) {
			const c = ensureGlobalColony(db);
			// Keep the colony alive so it doesn't reset and wipe the map.
			db.update(colonies)
				.set({
					resources: { ...c.resources, food: 110, water: 170 },
					lastPlayerActivityAt: Date.now(),
				})
				.where(eq(colonies._id, c._id))
				.run();
			advanceTime(db, 30);
			workerTick(db);
		}

		const outside = db
			.select()
			.from(worldTiles)
			.where(eq(worldTiles.colonyId, colony._id))
			.all()
			.filter(
				(t) =>
					Math.max(
						Math.abs(t.x - VILLAGE_ANCHOR.x),
						Math.abs(t.y - VILLAGE_ANCHOR.y),
					) > 4,
			);
		// Reveal spreads a halo of explored tiles (>62) well beyond the tiles a
		// cat's own feet touched — many tiles cross the bar as cats fan out.
		const revealed = outside.filter((t) => t.pathWear > 62);
		expect(revealed.length).toBeGreaterThan(20);
	});

	it("wears a repeatedly-trodden corridor into a visible road", () => {
		const colony = ensureGlobalColony(db);
		// Pin one cat to a den so the leader never reassigns it, then walk it up
		// and down a straight corridor south of the village. A second pass over
		// a tile crosses the road threshold (>=70).
		const walker = getAliveCatsForTest(db, colony._id)[0];
		const den = db
			.select()
			.from(buildingsTable)
			.where(eq(buildingsTable.colonyId, colony._id))
			.all()
			.find((b) => b.type === "den");
		db.update(cats)
			.set({ assignedBuildingId: den?._id ?? null })
			.where(eq(cats._id, walker._id))
			.run();

		const low = { map: "world" as const, x: 6, y: 16 };
		const high = { map: "world" as const, x: 6, y: 26 };
		db.update(cats)
			.set({ position: low, destination: high, activity: "traveling" })
			.where(eq(cats._id, walker._id))
			.run();

		for (let i = 0; i < 60; i++) {
			const cur = db.select().from(cats).where(eq(cats._id, walker._id)).get()!;
			// Keep it walking the corridor: flip target when it nears an end.
			const target = cur.position.y >= 24 ? low : high;
			db.update(cats)
				.set({ destination: target, activity: "traveling" })
				.where(eq(cats._id, walker._id))
				.run();
			// Keep the colony stocked so it never resets mid-test.
			const c = ensureGlobalColony(db);
			db.update(colonies)
				.set({
					resources: { ...c.resources, food: 180, water: 190 },
					lastPlayerActivityAt: Date.now(),
				})
				.where(eq(colonies._id, c._id))
				.run();
			advanceTime(db, 2);
			workerTick(db);
		}

		const corridor = db
			.select()
			.from(worldTiles)
			.where(eq(worldTiles.colonyId, colony._id))
			.all()
			.filter((t) => t.x === 6 && t.y >= 18 && t.y <= 24);
		const road = corridor.some((t) => t.pathWear >= 70);
		expect(road).toBe(true);
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

/** Trim the colony to `keep` cats, all healthy adults at `ageHours`. */
function foundSmallColony(
	database: GameDb,
	colonyId: string,
	keep: number,
	ageHours: number,
) {
	const all = getAliveCatsForTest(database, colonyId);
	for (const cat of all.slice(keep)) {
		database.delete(cats).where(eq(cats._id, cat._id)).run();
	}
	database
		.update(cats)
		.set({
			ageHours,
			isPregnant: false,
			pregnancyDueAgeHours: null,
			pregnancyMateId: null,
			needs: { hunger: 100, thirst: 100, rest: 100, health: 100 },
		})
		.where(eq(cats.colonyId, colonyId))
		.run();
}

describe("life simulation", () => {
	it("grows a bounded, aging population with births and new life stages", () => {
		const colony = ensureGlobalColony(db);
		setTestRngSeed(db, 4242);

		// A small, healthy founding group with clear housing headroom (shrine +
		// 5 dens shelter 14). Adults just under elderhood so aging shows without
		// wiping the founders mid-run.
		foundSmallColony(db, colony._id, 8, 26);
		db.update(colonies)
			.set({ testTimeScale: 60 })
			.where(eq(colonies._id, colony._id))
			.run();

		const housingCap = 14;
		const founders = 8;
		let peak = founders;
		let kittenSeen = false;

		// ~24 game-hours (advanceTime 60s * timeScale 60 = 1 game-hour/tick).
		for (let i = 0; i < 24; i += 1) {
			// Keep the stores full so only old age — never starvation — removes
			// cats; breeding needs the colony fed and watered.
			const current = ensureGlobalColony(db);
			db.update(colonies)
				.set({
					resources: { ...current.resources, food: 100_000, water: 100_000 },
				})
				.where(eq(colonies._id, current._id))
				.run();

			advanceTime(db, 60);
			workerTick(db);

			const living = getAliveCatsForTest(db, colony._id);
			peak = Math.max(peak, living.length);
			if (living.some((cat) => getLifeStage(cat.ageHours ?? 0) === "kitten")) {
				kittenSeen = true;
			}
			// Never runs away past the housing soft-cap (+ slack), never empties.
			expect(living.length).toBeGreaterThanOrEqual(1);
			expect(living.length).toBeLessThanOrEqual(housingCap + 4);
		}

		const birthEvents = db
			.select()
			.from(events)
			.where(and(eq(events.colonyId, colony._id), eq(events.type, "birth")))
			.all();

		// Births happened, kittens existed, and the colony grew past its founders.
		expect(birthEvents.length).toBeGreaterThan(0);
		expect(kittenSeen).toBe(true);
		expect(peak).toBeGreaterThan(founders);

		// Kittens inherit both parents (the co-parent is recorded).
		const born = getAliveCatsForTest(db, colony._id).filter(
			(cat) => cat.parentIds[0] !== null,
		);
		expect(born.length).toBeGreaterThan(0);
		expect(born.some((cat) => cat.parentIds[1] !== null)).toBe(true);
	});

	it("retires an elder of old age and frees the job it was on", () => {
		const colony = ensureGlobalColony(db);
		setTestRngSeed(db, 1);
		const elder = getAliveCatsForTest(db, colony._id)[0];

		// Ancient cat, mid-expedition.
		db.update(cats).set({ ageHours: 400 }).where(eq(cats._id, elder._id)).run();
		const now = Date.now();
		db.insert(jobs)
			.values({
				_id: "elder-hunt",
				colonyId: colony._id,
				kind: "hunt_expedition",
				status: "active",
				requestedByType: "leader",
				requestedByPlayerId: null,
				assignedCatId: elder._id,
				baseDurationSec: 8 * 3600,
				speedMultiplier: 1,
				yieldMultiplier: 1,
				clickTimeReducedSec: 0,
				createdAt: now,
				startedAt: now,
				endsAt: now + 8 * 3600 * 1000,
				metadata: {},
			})
			.run();

		// A game-hour at 400h age carries a near-certain death chance.
		db.update(colonies)
			.set({ testTimeScale: 60 })
			.where(eq(colonies._id, colony._id))
			.run();
		advanceTime(db, 60);
		workerTick(db);

		const row = db.select().from(cats).where(eq(cats._id, elder._id)).get()!;
		expect(row.deathTime).not.toBeNull();

		const job = db.select().from(jobs).where(eq(jobs._id, "elder-hunt")).get()!;
		expect(job.status).toBe("cancelled");

		const oldAgeDeaths = db
			.select()
			.from(events)
			.where(and(eq(events.colonyId, colony._id), eq(events.type, "death")))
			.all()
			.filter((event) => event.involvedCatIds.includes(elder._id));
		expect(oldAgeDeaths.length).toBeGreaterThan(0);
	});

	it("suffers starvation deaths in a food crisis and stops once restored", () => {
		const colony = ensureGlobalColony(db);
		foundSmallColony(db, colony._id, 10, 30);

		// Crisis: empty stores. Four cats are at death's door; the rest are hale
		// and will ride the famine out.
		db.update(colonies)
			.set({ resources: { ...colony.resources, food: 0, water: 0 } })
			.where(eq(colonies._id, colony._id))
			.run();
		const roster = getAliveCatsForTest(db, colony._id);
		for (const cat of roster.slice(0, 4)) {
			db.update(cats)
				.set({ needs: { hunger: 0, thirst: 0, rest: 50, health: 0.5 } })
				.where(eq(cats._id, cat._id))
				.run();
		}

		advanceTime(db, 60);
		workerTick(db);

		const afterCrisis = getAliveCatsForTest(db, colony._id).length;
		const deathEvents = db
			.select()
			.from(events)
			.where(and(eq(events.colonyId, colony._id), eq(events.type, "death")))
			.all();
		expect(deathEvents.length).toBeGreaterThan(0);
		expect(afterCrisis).toBeLessThan(10);
		expect(afterCrisis).toBeGreaterThan(0);

		// Relief arrives: restock and let the survivors recover without dying off.
		const survivor = ensureGlobalColony(db);
		db.update(colonies)
			.set({ resources: { ...survivor.resources, food: 100, water: 100 } })
			.where(eq(colonies._id, survivor._id))
			.run();
		db.update(cats)
			.set({ needs: { hunger: 100, thirst: 100, rest: 100, health: 100 } })
			.where(and(eq(cats.colonyId, colony._id), isNull(cats.deathTime)))
			.run();

		advanceTime(db, 60);
		workerTick(db);
		expect(getAliveCatsForTest(db, colony._id).length).toBe(afterCrisis);
	});
});
