import { and, eq } from "drizzle-orm";
import { beforeEach, describe, expect, it } from "vitest";

import { createDb, type GameDb } from "@/db/client";
import { cats, colonies, worldTiles } from "@/db/schema";
import { advanceTime, ensureGlobalColony, workerTick } from "@/server/game";

let db: GameDb;

beforeEach(() => {
	db = createDb(":memory:");
});

function prepareColonyForDecay(database: GameDb): string {
	const colony = ensureGlobalColony(database);
	database
		.update(cats)
		.set({ activity: "working", destination: null, currentTask: null })
		.where(eq(cats.colonyId, colony._id))
		.run();
	database
		.update(colonies)
		.set({
			resources: {
				food: 0,
				water: 0,
				herbs: 0,
				materials: 0,
				blessings: 0,
				refined: 0,
				weapons: 0,
				armor: 0,
			},
			testRngSeed: 12345,
		})
		.where(eq(colonies._id, colony._id))
		.run();
	return colony._id;
}

function insertWearTile(
	database: GameDb,
	colonyId: string,
	label: string,
	x: number,
	pathWear: number,
	overlayFeature: string | null = null,
): void {
	database
		.insert(worldTiles)
		.values({
			_id: `path-wear-decay-${label}`,
			colonyId,
			x,
			y: 9000,
			type: "field",
			resources: { food: 0, herbs: 0, water: 0 },
			maxResources: { food: 0, herbs: 0 },
			dangerLevel: 0,
			pathWear,
			lastDepleted: 0,
			overlayFeature,
		})
		.run();
}

function wearAt(database: GameDb, colonyId: string, x: number): number {
	return database
		.select({ pathWear: worldTiles.pathWear })
		.from(worldTiles)
		.where(
			and(
				eq(worldTiles.colonyId, colonyId),
				eq(worldTiles.x, x),
				eq(worldTiles.y, 9000),
			),
		)
		.get()!.pathWear;
}

describe("path-wear decay", () => {
	it("preserves the existing decay thresholds and paved-road exemption", () => {
		const colonyId = prepareColonyForDecay(db);
		insertWearTile(db, colonyId, "zero", 9000, 0);
		insertWearTile(db, colonyId, "below-floor", 9001, 0.5);
		insertWearTile(db, colonyId, "floor", 9002, 1);
		insertWearTile(db, colonyId, "faint", 9003, 45);
		insertWearTile(db, colonyId, "revealed", 9004, 63);
		insertWearTile(db, colonyId, "almost-road", 9005, 69);
		insertWearTile(db, colonyId, "road-threshold", 9006, 70);
		insertWearTile(db, colonyId, "road-grade", 9007, 80);
		insertWearTile(db, colonyId, "paved", 9008, 100, "road_built");

		advanceTime(db, 60);
		workerTick(db);

		expect(wearAt(db, colonyId, 9000)).toBe(0);
		expect(wearAt(db, colonyId, 9001)).toBe(1);
		expect(wearAt(db, colonyId, 9002)).toBe(1);
		expect(wearAt(db, colonyId, 9003)).toBe(44);
		expect(wearAt(db, colonyId, 9004)).toBe(63);
		expect(wearAt(db, colonyId, 9005)).toBe(69);
		expect(wearAt(db, colonyId, 9006)).toBe(69);
		expect(wearAt(db, colonyId, 9007)).toBe(79);
		expect(wearAt(db, colonyId, 9008)).toBe(100);
	});

	it("is deterministic for identical path-wear state", () => {
		const left = createDb(":memory:");
		const right = createDb(":memory:");
		const leftColony = prepareColonyForDecay(left);
		const rightColony = prepareColonyForDecay(right);
		for (const database of [left, right]) {
			const colonyId = database === left ? leftColony : rightColony;
			insertWearTile(database, colonyId, "faint", 9010, 45);
			insertWearTile(database, colonyId, "revealed", 9011, 63);
			insertWearTile(database, colonyId, "road-grade", 9012, 80);
			insertWearTile(database, colonyId, "paved", 9013, 100, "road_built");
			advanceTime(database, 60);
			workerTick(database);
		}

		const digest = (database: GameDb, colonyId: string) =>
			[9010, 9011, 9012, 9013]
				.map((x) => wearAt(database, colonyId, x))
				.join(",");

		expect(digest(right, rightColony)).toBe(digest(left, leftColony));
	});
});
