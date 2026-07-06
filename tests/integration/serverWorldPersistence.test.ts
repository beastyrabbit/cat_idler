/**
 * World-persistence regression tests (issue: "the map resets with the village").
 *
 * When a colony collapses, the run resets — cats, resources, jobs and any
 * in-progress construction start over — but the WORLD must persist: the same
 * colony row (so worldTiles are never orphaned), the same worldSeed (so the
 * client regenerates identical terrain), and the same explored pathWear / fog
 * on every worldTile. Only completed buildings and the shrine survive as the
 * standing village; half-built structures are cleared.
 */

import { and, eq } from "drizzle-orm";
import { nanoid } from "nanoid";
import { beforeEach, describe, expect, it } from "vitest";
import { createDb, type GameDb } from "@/db/client";
import { buildings, cats, colonies, runHistory, worldTiles } from "@/db/schema";
import { advanceTime, ensureGlobalColony, workerTick } from "@/server/game";

let db: GameDb;

beforeEach(() => {
	db = createDb(":memory:");
});

/** Drain stores and push every cat to the brink so the survival pass wipes the
 * colony mid-tick, triggering resetGlobalRun with reason "all-cats-dead". */
function collapseColony(colonyId: string) {
	const colony = db
		.select()
		.from(colonies)
		.where(eq(colonies._id, colonyId))
		.get()!;
	db.update(colonies)
		.set({ resources: { ...colony.resources, food: 0, water: 0 } })
		.where(eq(colonies._id, colonyId))
		.run();
	db.update(cats)
		.set({ needs: { hunger: 0, thirst: 0, rest: 50, health: 1 } })
		.where(eq(cats.colonyId, colonyId))
		.run();
	advanceTime(db, 3600);
	workerTick(db);
}

describe("world persistence across a colony collapse", () => {
	it("keeps the same colony, worldSeed and explored terrain, and only rerolls the village", () => {
		const before = ensureGlobalColony(db);

		// Baseline world snapshot.
		const tilesBefore = db
			.select()
			.from(worldTiles)
			.where(eq(worldTiles.colonyId, before._id))
			.all();
		expect(tilesBefore.length).toBeGreaterThan(0);

		// Mark a tile as explored (pathWear) so we can prove exploration survives.
		const markedTile = tilesBefore[0];
		db.update(worldTiles)
			.set({ pathWear: 77 })
			.where(eq(worldTiles._id, markedTile._id))
			.run();

		// A completed den (should survive) and a half-built den (should be cleared).
		const completedDenId = nanoid();
		const inProgressDenId = nanoid();
		db.insert(buildings)
			.values({
				_id: completedDenId,
				colonyId: before._id,
				type: "den",
				level: 1,
				position: { x: 3, y: 3 },
				constructionProgress: 100,
			})
			.run();
		db.insert(buildings)
			.values({
				_id: inProgressDenId,
				colonyId: before._id,
				type: "den",
				level: 1,
				position: { x: 4, y: 4 },
				constructionProgress: 40,
			})
			.run();

		collapseColony(before._id);

		// The run reset actually happened.
		const history = db.select().from(runHistory).all();
		expect(history).toHaveLength(1);
		const after = ensureGlobalColony(db);
		expect(after.runNumber).toBe((before.runNumber ?? 1) + 1);

		// Same colony row — no new colonyId that would orphan the worldTiles.
		expect(after._id).toBe(before._id);
		// Same terrain seed — the client regenerates the identical map.
		expect(after.worldSeed).toBe(before.worldSeed);

		// worldTiles are untouched: same rows, and the explored pathWear persists.
		const tilesAfter = db
			.select()
			.from(worldTiles)
			.where(eq(worldTiles.colonyId, before._id))
			.all();
		expect(tilesAfter.length).toBe(tilesBefore.length);
		const markedAfter = db
			.select()
			.from(worldTiles)
			.where(eq(worldTiles._id, markedTile._id))
			.get()!;
		// Exploration survives the reset. pathWear fades slowly over time on its
		// own, so it need not be exactly 77 — the point is the collapse did not
		// wipe it back toward 0.
		expect(markedAfter.pathWear).toBeGreaterThan(60);

		// Completed buildings (and the shrine) survive; in-progress ones are cleared.
		const completedAfter = db
			.select()
			.from(buildings)
			.where(eq(buildings._id, completedDenId))
			.get();
		expect(completedAfter).toBeTruthy();
		const inProgressAfter = db
			.select()
			.from(buildings)
			.where(eq(buildings._id, inProgressDenId))
			.get();
		expect(inProgressAfter).toBeUndefined();
		const shrineAfter = db
			.select()
			.from(buildings)
			.where(
				and(eq(buildings.colonyId, before._id), eq(buildings.type, "shrine")),
			)
			.get();
		expect(shrineAfter).toBeTruthy();
	});
});
