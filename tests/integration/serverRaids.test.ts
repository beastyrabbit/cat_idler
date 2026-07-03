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
import { cats, colonies, events, raiders } from "@/db/schema";
import { MAX_RAID_CASUALTIES } from "@/lib/game/threat";
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
});
