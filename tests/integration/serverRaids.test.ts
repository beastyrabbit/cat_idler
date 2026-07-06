/**
 * Integration: the military raid loop against in-memory SQLite.
 *
 * Forces a raid at the gate and ticks once to resolve it, checking the balance
 * contract: a DEFAULT bootstrap colony (20 cats, no warriors, no gear) turns
 * its opening raid away as a militia with no deaths; an equipped guard drives a
 * warband off spending only gear; and only a genuinely overwhelming warband
 * sacks the village — and even then the loss is capped (at most one death,
 * bounded theft), so the colony lives to rebuild. Determinism is pinned under
 * setTestRngSeed.
 */

import { and, eq, isNull } from "drizzle-orm";
import { beforeEach, describe, expect, it } from "vitest";

import { createDb, type GameDb } from "@/db/client";
import { type CatRow, cats, colonies, events, jobs, raiders } from "@/db/schema";
import { MAX_RAID_CASUALTIES } from "@/lib/game/threat";
import { VILLAGE_ANCHOR } from "@/lib/game/villageLayout";
import {
	advanceTime,
	ensureGlobalColony,
	setTestRngSeed,
	spawnRaidForTest,
	workerTick,
} from "@/server/game";

let db: GameDb;

beforeEach(() => {
	db = createDb(":memory:");
});

function colonyId(): string {
	return ensureGlobalColony(db)._id;
}

function makeWarriors(count: number, attack = 60, defense = 60) {
	const id = colonyId();
	const roster = db
		.select()
		.from(cats)
		.where(and(eq(cats.colonyId, id), isNull(cats.deathTime)))
		.all();
	for (let i = 0; i < count && i < roster.length; i += 1) {
		const cat = roster[i];
		db.update(cats)
			.set({
				specialization: "warrior",
				ageHours: 30,
				stats: { ...cat.stats, attack, defense },
			})
			.where(eq(cats._id, cat._id))
			.run();
	}
}

function setResources(patch: Record<string, number>) {
	const colony = ensureGlobalColony(db);
	db.update(colonies)
		.set({ resources: { ...colony.resources, ...patch } })
		.where(eq(colonies._id, colony._id))
		.run();
}

function livingCats(): number {
	const id = colonyId();
	return db
		.select()
		.from(cats)
		.where(and(eq(cats.colonyId, id), isNull(cats.deathTime)))
		.all().length;
}

function eventTypes(): string[] {
	const id = colonyId();
	return db
		.select()
		.from(events)
		.where(eq(events.colonyId, id))
		.all()
		.map((e) => e.type);
}

function ambushEvents(database = db) {
	const id = ensureGlobalColony(database)._id;
	return database
		.select()
		.from(events)
		.where(and(eq(events.colonyId, id), eq(events.type, "raider_ambush")))
		.all();
}

function firstLivingCat(database = db) {
	const id = ensureGlobalColony(database)._id;
	const cat = database
		.select()
		.from(cats)
		.where(and(eq(cats.colonyId, id), isNull(cats.deathTime)))
		.all()[0];
	if (!cat) throw new Error("expected living cat");
	return cat;
}

function placeCatForAmbush(catId: string, patch: Partial<CatRow> = {}) {
	db.update(cats)
		.set({
			position: { map: "world", x: VILLAGE_ANCHOR.x + 8, y: VILLAGE_ANCHOR.y },
			destination: {
				map: "world",
				x: VILLAGE_ANCHOR.x + 12,
				y: VILLAGE_ANCHOR.y,
			},
			activity: "traveling",
			currentTask: "hunt_expedition",
			ageHours: 30,
			stats: {
				attack: 50,
				defense: 50,
				hunting: 50,
				medicine: 50,
				cleaning: 50,
				building: 50,
				leadership: 50,
				vision: 50,
			},
			...patch,
		})
		.where(eq(cats._id, catId))
		.run();
}

function insertActiveFieldJob(catId: string, now = Date.now()) {
	const id = colonyId();
	db.insert(jobs)
		.values({
			_id: `job-${catId}`,
			colonyId: id,
			kind: "hunt_expedition",
			status: "active",
			requestedByType: "leader",
			requestedByPlayerId: null,
			assignedCatId: catId,
			baseDurationSec: 3600,
			speedMultiplier: 1,
			yieldMultiplier: 1,
			clickTimeReducedSec: 0,
			createdAt: now,
			startedAt: now,
			endsAt: now + 3600_000,
			metadata: {
				site: { x: VILLAGE_ANCHOR.x + 12, y: VILLAGE_ANCHOR.y },
				accepted: true,
			},
		})
		.run();
}

function forceAdvancingRaid(count = 1, strength = 40) {
	const spawn = spawnRaidForTest(db, { atGate: false, count, strength });
	expect(spawn.ok).toBe(true);
	const id = colonyId();
	const units = db
		.select()
		.from(raiders)
		.where(eq(raiders.colonyId, id))
		.all();
	for (let i = 0; i < units.length; i += 1) {
		db.update(raiders)
			.set({
				position: {
					x: VILLAGE_ANCHOR.x + 9,
					y: VILLAGE_ANCHOR.y + i,
				},
				status: "advancing",
			})
			.where(eq(raiders._id, units[i]._id))
			.run();
	}
}

describe("raid resolution", () => {
	it("a default bootstrap colony repels its first raid with zero deaths", () => {
		// The balance contract: a fresh colony — 20 cats, starter stores, ZERO
		// warriors, ZERO gear — turns its opening warband away as a militia, and
		// nobody dies. Spawned with no plan override so planRaid sizes the real
		// opening raid (a lone weak raider) from the starter snapshot.
		ensureGlobalColony(db);
		setTestRngSeed(db, 4242);

		const before = livingCats();
		const spawn = spawnRaidForTest(db, { atGate: true });
		expect(spawn.ok).toBe(true);

		advanceTime(db, 30);
		workerTick(db);

		const colony = ensureGlobalColony(db);
		expect(colony.activeRaidId).toBeNull();
		expect(
			db.select().from(raiders).where(eq(raiders.colonyId, colony._id)).all(),
		).toHaveLength(0);
		// The militia held the gate — no casualties.
		expect(livingCats()).toBe(before);
		expect(eventTypes()).toContain("raid_repelled");
		expect(eventTypes()).not.toContain("raid_sacked");
	});

	it("equipped warriors drive off a raid spending only gear", () => {
		ensureGlobalColony(db);
		setTestRngSeed(db, 4242);
		makeWarriors(3);
		setResources({ weapons: 5, armor: 5, food: 150, water: 150 });

		const before = livingCats();
		const spawn = spawnRaidForTest(db, {
			atGate: true,
			count: 2,
			strength: 30,
		});
		expect(spawn.ok).toBe(true);

		advanceTime(db, 30);
		workerTick(db);

		const colony = ensureGlobalColony(db);
		// Raid resolved and cleared.
		expect(colony.activeRaidId).toBeNull();
		expect(
			db.select().from(raiders).where(eq(raiders.colonyId, colony._id)).all(),
		).toHaveLength(0);
		// The guard held — nobody died.
		expect(livingCats()).toBe(before);
		// Gear is a consumable: with 20 defenders and 5 of each, the muster spends
		// all of it (warriors first, then militia draw the leftovers).
		expect(colony.resources.weapons).toBe(0);
		expect(colony.resources.armor).toBe(0);
		expect(eventTypes()).toContain("raid_repelled");
	});

	it("only an overwhelming warband sacks the village, and the loss is capped", () => {
		ensureGlobalColony(db);
		setTestRngSeed(db, 4242);
		// A warband far beyond what 20 militia can hold — the only way past the
		// gate now that everyone fights.
		setResources({ food: 300, materials: 200, weapons: 0, armor: 0 });

		const foodBefore = ensureGlobalColony(db).resources.food;
		const before = livingCats();
		const spawn = spawnRaidForTest(db, {
			atGate: true,
			count: 12,
			strength: 220,
		});
		expect(spawn.ok).toBe(true);

		advanceTime(db, 30);
		workerTick(db);

		const colony = ensureGlobalColony(db);
		expect(colony.activeRaidId).toBeNull();
		// Stores were looted — food dropped well past what a 30s tick consumes.
		expect(colony.resources.food).toBeLessThan(foodBefore - 20);
		// ...but bounded: the raiders leave the majority of the store behind.
		expect(colony.resources.food).toBeGreaterThan(foodBefore * 0.6);
		// Casualties are capped — one bad fight never wipes the colony.
		expect(livingCats()).toBeGreaterThanOrEqual(before - MAX_RAID_CASUALTIES);
		expect(eventTypes()).toContain("raid_sacked");
	});

	it("survives a long run of raids from bootstrap without player help", () => {
		// Balance contract: repeated raids must never cascade the colony to death.
		// Force 40 back-to-back raids on a default colony (no warriors, no gear),
		// keeping the larder full so neglect can't reset the run — the only thing
		// that can kill a cat here is a raid. The militia turns every one away.
		ensureGlobalColony(db);
		setTestRngSeed(db, 4242);

		const before = livingCats();
		let raidsRun = 0;
		let resets = 0;
		for (let i = 0; i < 40; i += 1) {
			setResources({ food: 400, water: 400 });
			const spawn = spawnRaidForTest(db, { atGate: true }) as { ok: boolean };
			if (spawn.ok) raidsRun += 1;
			advanceTime(db, 30);
			const res = workerTick(db) as { reset?: boolean };
			if (res?.reset) resets += 1;
			expect(livingCats()).toBeGreaterThan(0);
		}

		expect(raidsRun).toBe(40);
		expect(resets).toBe(0);
		// Every raid was turned away by the militia — nobody died to a raid.
		expect(livingCats()).toBe(before);
		expect(eventTypes()).toContain("raid_repelled");
		expect(eventTypes()).not.toContain("raid_sacked");
		expect(eventTypes()).not.toContain("raid_casualty");
	});

	it("is deterministic under the same seed", () => {
		const run = () => {
			const local = createDb(":memory:");
			ensureGlobalColony(local);
			setTestRngSeed(local, 99);
			const colony = ensureGlobalColony(local);
			// Pin cat stats so the muster is deterministic (starter stats are rolled
			// from unseeded Math.random); the raid roll is already seeded.
			for (const cat of local
				.select()
				.from(cats)
				.where(and(eq(cats.colonyId, colony._id), isNull(cats.deathTime)))
				.all()) {
				local
					.update(cats)
					.set({ stats: { ...cat.stats, attack: 40, defense: 40 } })
					.where(eq(cats._id, cat._id))
					.run();
			}
			local
				.update(colonies)
				.set({ resources: { ...colony.resources, food: 300 } })
				.where(eq(colonies._id, colony._id))
				.run();
			spawnRaidForTest(local, { atGate: true, count: 12, strength: 220 });
			advanceTime(local, 30);
			workerTick(local);
			return ensureGlobalColony(local).resources.food;
		};
		expect(run()).toBe(run());
	});

	it("intercepts a traveler crossing an advancing warband path", () => {
		ensureGlobalColony(db);
		setTestRngSeed(db, 4242);
		const traveler = firstLivingCat();
		placeCatForAmbush(traveler._id);
		insertActiveFieldJob(traveler._id);
		forceAdvancingRaid(1, 40);

		advanceTime(db, 1);
		workerTick(db);

		const ambush = ambushEvents();
		expect(ambush).toHaveLength(1);
		expect(ambush[0].involvedCatIds).toContain(traveler._id);
		expect((ambush[0].metadata as Record<string, unknown>).outcome).toBe(
			"flee",
		);
	});

	it("does not intercept a traveler behind the fence", () => {
		ensureGlobalColony(db);
		setTestRngSeed(db, 4242);
		const traveler = firstLivingCat();
		placeCatForAmbush(traveler._id, {
			position: { map: "world", ...VILLAGE_ANCHOR },
			destination: {
				map: "world",
				x: VILLAGE_ANCHOR.x + 1,
				y: VILLAGE_ANCHOR.y,
			},
		});
		insertActiveFieldJob(traveler._id);
		forceAdvancingRaid(1, 40);
		db.update(raiders)
			.set({
				position: { x: VILLAGE_ANCHOR.x + 1, y: VILLAGE_ANCHOR.y },
				status: "advancing",
			})
			.where(eq(raiders.colonyId, colonyId()))
			.run();

		advanceTime(db, 1);
		workerTick(db);

		expect(ambushEvents()).toHaveLength(0);
	});

	it("drops carried yield and cancels field work when a cat flees", () => {
		ensureGlobalColony(db);
		setTestRngSeed(db, 4242);
		const traveler = firstLivingCat();
		placeCatForAmbush(traveler._id, {
			activity: "returning",
			carrying: { kind: "food", amount: 7, jobEndedAt: Date.now() },
		});
		insertActiveFieldJob(traveler._id);
		forceAdvancingRaid(1, 40);

		advanceTime(db, 1);
		workerTick(db);

		const after = db
			.select()
			.from(cats)
			.where(eq(cats._id, traveler._id))
			.get();
		const job = db
			.select()
			.from(jobs)
			.where(eq(jobs.assignedCatId, traveler._id))
			.get();
		const ambush = ambushEvents()[0];
		expect(after?.carrying).toBeNull();
		expect(after?.activity).toBe("returning");
		expect(after?.destination).toEqual({ map: "world", ...VILLAGE_ANCHOR });
		expect(job?.status).toBe("cancelled");
		expect((ambush.metadata as Record<string, unknown>).dropped).toEqual({
			kind: "food",
			amount: 7,
		});
	});

	it("counts interception kills against the raid casualty cap", () => {
		ensureGlobalColony(db);
		setTestRngSeed(db, 4242);
		const roster = db
			.select()
			.from(cats)
			.where(and(eq(cats.colonyId, colonyId()), isNull(cats.deathTime)))
			.all();
		const first = roster[0];
		const second = roster[1];
		placeCatForAmbush(first._id, {
			position: { map: "world", x: VILLAGE_ANCHOR.x + 8, y: VILLAGE_ANCHOR.y },
		});
		placeCatForAmbush(second._id, {
			position: {
				map: "world",
				x: VILLAGE_ANCHOR.x + 8,
				y: VILLAGE_ANCHOR.y + 1,
			},
		});
		insertActiveFieldJob(first._id);
		insertActiveFieldJob(second._id);
		const before = livingCats();
		forceAdvancingRaid(2, 300);

		advanceTime(db, 1);
		workerTick(db);

		const outcomes = ambushEvents().map(
			(e) => (e.metadata as Record<string, unknown>).outcome,
		);
		expect(livingCats()).toBe(before - MAX_RAID_CASUALTIES);
		expect(outcomes.filter((outcome) => outcome === "killed")).toHaveLength(1);
		expect(outcomes.filter((outcome) => outcome === "wounded")).toHaveLength(1);
	});

	it("is deterministic for identical seeded interception runs", () => {
		const run = () => {
			const local = createDb(":memory:");
			db = local;
			ensureGlobalColony(local);
			setTestRngSeed(local, 4242);
			const traveler = firstLivingCat(local);
			placeCatForAmbush(traveler._id);
			insertActiveFieldJob(traveler._id);
			forceAdvancingRaid(1, 40);
			advanceTime(local, 1);
			workerTick(local);
			const after = local
				.select()
				.from(cats)
				.where(eq(cats._id, traveler._id))
				.get();
			const event = ambushEvents(local)[0];
			return {
				message: event.message,
				outcome: (event.metadata as Record<string, unknown>).outcome,
				activity: after?.activity,
				carrying: after?.carrying,
				deathTime: after?.deathTime == null ? null : "dead",
			};
		};

		expect(run()).toEqual(run());
	});
});
