/**
 * Integration: the military raid loop against in-memory SQLite.
 *
 * Forces a raid at the gate and ticks once to resolve it, checking both
 * outcomes: an equipped guard drives the warband off with no losses beyond
 * the consumed gear, while a defenceless colony is sacked and loses stores.
 * Determinism is pinned under setTestRngSeed.
 */

import { and, eq, isNull } from "drizzle-orm";
import { beforeEach, describe, expect, it } from "vitest";

import { createDb, type GameDb } from "@/db/client";
import { cats, colonies, events, raiders } from "@/db/schema";
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

describe("raid resolution", () => {
	it("equipped warriors drive off a raid with no losses beyond gear", () => {
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
		// Gear was spent defending (3 warriors each drew a weapon + armor).
		expect(colony.resources.weapons).toBe(2);
		expect(colony.resources.armor).toBe(2);
		expect(eventTypes()).toContain("raid_repelled");
	});

	it("a defenceless colony is sacked and loses stores", () => {
		ensureGlobalColony(db);
		setTestRngSeed(db, 4242);
		setResources({ food: 300, materials: 200, weapons: 0, armor: 0 });

		const foodBefore = ensureGlobalColony(db).resources.food;
		const spawn = spawnRaidForTest(db, {
			atGate: true,
			count: 6,
			strength: 60,
		});
		expect(spawn.ok).toBe(true);

		advanceTime(db, 30);
		workerTick(db);

		const colony = ensureGlobalColony(db);
		expect(colony.activeRaidId).toBeNull();
		// Stores were looted — food dropped well past what a 30s tick consumes.
		expect(colony.resources.food).toBeLessThan(foodBefore - 20);
		expect(eventTypes()).toContain("raid_sacked");
	});

	it("is deterministic under the same seed", () => {
		const run = () => {
			const local = createDb(":memory:");
			ensureGlobalColony(local);
			setTestRngSeed(local, 99);
			const colony = ensureGlobalColony(local);
			local
				.update(colonies)
				.set({ resources: { ...colony.resources, food: 300 } })
				.where(eq(colonies._id, colony._id))
				.run();
			spawnRaidForTest(local, { atGate: true, count: 6, strength: 60 });
			advanceTime(local, 30);
			workerTick(local);
			return ensureGlobalColony(local).resources.food;
		};
		expect(run()).toBe(run());
	});
});
