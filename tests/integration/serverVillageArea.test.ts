/**
 * Integration tests for the organic village wiring (task #31 stage 2):
 * persistence of the claimed footprint, one-tile job-based growth, build-site
 * selection, and palisade pathing/cache invalidation.
 */

import { and, eq, isNull } from "drizzle-orm";
import { nanoid } from "nanoid";
import { beforeEach, describe, expect, it } from "vitest";
import { createDb, type GameDb } from "@/db/client";
import { buildings, cats, colonies, jobs, worldTiles } from "@/db/schema";
import { CHOPPED_FOREST_FOOD_CAP } from "@/lib/game/depletion";
import { buildColonyWalkGrid, findPath } from "@/lib/game/pathfinding";
import {
	expandVillage,
	fencePerimeter,
	fromTiles,
	gatePlacement,
	isInsideVillage,
	toTiles,
} from "@/lib/game/villageArea";
import { VILLAGE_ANCHOR } from "@/lib/game/villageLayout";
import {
	advanceTime,
	ensureGlobalColony,
	getGlobalDashboard,
	setTestRngSeed,
	workerTick,
} from "@/server/game";

let db: GameDb;
beforeEach(() => {
	db = createDb(":memory:");
});

function getColony() {
	const colony = db
		.select()
		.from(colonies)
		.where(eq(colonies.isGlobal, true))
		.get();
	if (!colony) {
		throw new Error("Expected global colony");
	}
	return colony;
}

function must<T>(value: T | null | undefined, label: string): T {
	if (value == null) {
		throw new Error(`Expected ${label}`);
	}
	return value;
}

function tightArea(radius: number) {
	const tiles = [];
	for (let dy = -radius; dy <= radius; dy++) {
		for (let dx = -radius; dx <= radius; dx++) {
			tiles.push({ x: VILLAGE_ANCHOR.x + dx, y: VILLAGE_ANCHOR.y + dy });
		}
	}
	return tiles;
}

function activeOrQueuedExpansionJobs(colonyId: string) {
	return db
		.select()
		.from(jobs)
		.where(eq(jobs.colonyId, colonyId))
		.all()
		.filter(
			(job) =>
				job.kind === "expand_village" &&
				(job.status === "active" || job.status === "queued"),
		);
}

function flattenWorld(colonyId: string) {
	db.update(worldTiles)
		.set({
			type: "field",
			overlayFeature: null,
			resources: { food: 0, herbs: 0, water: 0 },
			maxResources: { food: 40, herbs: 10 },
			pathWear: 0,
		})
		.where(eq(worldTiles.colonyId, colonyId))
		.run();
}

describe("claimed village area persistence", () => {
	it("seeds the founding square on a fresh colony and matches the historical gate", () => {
		ensureGlobalColony(db);
		const claimed = must(getColony().claimedTiles, "claimed tiles");
		expect(claimed).toHaveLength(49);

		const area = fromTiles(claimed);
		expect(isInsideVillage(VILLAGE_ANCHOR, area)).toBe(true);
		expect(
			isInsideVillage({ x: VILLAGE_ANCHOR.x + 3, y: VILLAGE_ANCHOR.y }, area),
		).toBe(true);
		expect(
			isInsideVillage({ x: VILLAGE_ANCHOR.x + 4, y: VILLAGE_ANCHOR.y }, area),
		).toBe(false);
		expect(gatePlacement(area)).toEqual({
			x: VILLAGE_ANCHOR.x,
			y: VILLAGE_ANCHOR.y + 3,
			side: "S",
		});
		expect(fencePerimeter(area)).toHaveLength(28);
	});

	it("preserves a grown claimed list across colony reload", () => {
		const colony = ensureGlobalColony(db);
		const grown = [
			...must(getColony().claimedTiles, "claimed tiles"),
			{ x: 2, y: 6 },
		];
		db.update(colonies)
			.set({ claimedTiles: grown })
			.where(eq(colonies._id, colony._id))
			.run();

		ensureGlobalColony(db);
		expect(getColony().claimedTiles).toEqual(grown);
		expect(getGlobalDashboard(db)?.claimedTiles).toEqual(
			toTiles(fromTiles(grown)),
		);
	});
});

describe("organic village growth from the tick", () => {
	it("queues a small expansion job when the claimed footprint is crowded", () => {
		const colony = ensureGlobalColony(db);
		setTestRngSeed(db, 7);
		const tight = tightArea(1);
		db.update(colonies)
			.set({ claimedTiles: tight })
			.where(eq(colonies._id, colony._id))
			.run();

		for (let i = 0; i < 12; i++) {
			advanceTime(db, 30);
			workerTick(db);
			if (activeOrQueuedExpansionJobs(colony._id).length > 0) break;
		}

		const expansion = activeOrQueuedExpansionJobs(colony._id)[0];
		expect(expansion).toBeDefined();
		expect(expansion.assignedCatId).toBeTruthy();
		expect(
			(expansion.metadata as { target?: { x: number; y: number } }).target,
		).toEqual(expandVillage(fromTiles(tight)));
		expect(getColony().claimedTiles).toHaveLength(tight.length);
	});

	it("claims exactly the recorded target on completion and clears forest there", () => {
		const colony = ensureGlobalColony(db);
		flattenWorld(colony._id);
		const tight = tightArea(1);
		const target = must(expandVillage(fromTiles(tight)), "expansion target");
		db.update(colonies)
			.set({ claimedTiles: tight })
			.where(eq(colonies._id, colony._id))
			.run();
		db.update(worldTiles)
			.set({
				type: "forest",
				resources: { food: 30, herbs: 4, water: 0 },
				maxResources: { food: 40, herbs: 10 },
				lastDepleted: 0,
			})
			.where(
				and(
					eq(worldTiles.colonyId, colony._id),
					eq(worldTiles.x, target.x),
					eq(worldTiles.y, target.y),
				),
			)
			.run();
		const worker = must(
			db
				.select()
				.from(cats)
				.where(and(eq(cats.colonyId, colony._id), isNull(cats.deathTime)))
				.get(),
			"worker cat",
		);
		const now = Date.now();
		db.insert(jobs)
			.values({
				_id: "expand-complete",
				colonyId: colony._id,
				kind: "expand_village",
				status: "active",
				requestedByType: "leader",
				requestedByPlayerId: null,
				assignedCatId: worker._id,
				baseDurationSec: 1,
				speedMultiplier: 1,
				yieldMultiplier: 1,
				clickTimeReducedSec: 0,
				createdAt: now - 2000,
				startedAt: now - 2000,
				endsAt: now - 1000,
				metadata: { target },
			})
			.run();

		advanceTime(db, 2);
		workerTick(db);

		const claimed = must(getColony().claimedTiles, "claimed tiles");
		expect(claimed).toHaveLength(tight.length + 1);
		expect(claimed).toContainEqual(target);
		const cleared = must(
			db
				.select()
				.from(worldTiles)
				.where(
					and(
						eq(worldTiles.colonyId, colony._id),
						eq(worldTiles.x, target.x),
						eq(worldTiles.y, target.y),
					),
				)
				.get(),
			"cleared tile",
		);
		expect(cleared.type).toBe("field");
		expect(cleared.resources.food).toBe(0);
		expect(cleared.resources.herbs).toBe(0);
		expect(cleared.maxResources.food).toBe(CHOPPED_FOREST_FOOD_CAP);
		expect(cleared.lastDepleted).toBeGreaterThan(0);
	});
});

describe("claimed-area construction sites", () => {
	it("places scaffolds only on free claimed tiles", () => {
		const colony = ensureGlobalColony(db);
		const claimed = [
			VILLAGE_ANCHOR,
			{ x: VILLAGE_ANCHOR.x + 1, y: VILLAGE_ANCHOR.y },
		];
		db.update(colonies)
			.set({ claimedTiles: claimed })
			.where(eq(colonies._id, colony._id))
			.run();
		for (const building of db
			.select()
			.from(buildings)
			.where(eq(buildings.colonyId, colony._id))
			.all()) {
			if (building.type !== "shrine") {
				db.delete(buildings).where(eq(buildings._id, building._id)).run();
			}
		}
		const architect = must(
			db
				.select()
				.from(cats)
				.where(and(eq(cats.colonyId, colony._id), isNull(cats.deathTime)))
				.get(),
			"architect cat",
		);
		const now = Date.now();
		db.insert(jobs)
			.values({
				_id: nanoid(),
				colonyId: colony._id,
				kind: "build_house",
				status: "queued",
				requestedByType: "leader",
				requestedByPlayerId: null,
				assignedCatId: architect._id,
				baseDurationSec: 100,
				speedMultiplier: 1,
				yieldMultiplier: 1,
				clickTimeReducedSec: 0,
				createdAt: now,
				startedAt: now,
				endsAt: now + 100_000,
				metadata: { phase: "construct_house", buildingType: "den" },
			})
			.run();

		advanceTime(db, 2);
		workerTick(db);

		const scaffold = db
			.select()
			.from(buildings)
			.where(eq(buildings.colonyId, colony._id))
			.all()
			.find((building) => building.constructionProgress < 100);
		expect(scaffold?.position).toEqual({ x: 1, y: 0 });
	});
});

describe("walkgrid palisade follows the claimed shape", () => {
	it("blocks crossing the boundary except at the gate", () => {
		const tiles = [];
		for (let dy = 0; dy <= 2; dy++) {
			for (let dx = 0; dx <= 2; dx++) tiles.push({ x: dx, y: dy });
		}
		const area = fromTiles(tiles);
		const gate = must(gatePlacement(area), "gate");
		const grid = buildColonyWalkGrid({
			tiles: [],
			anchor: { x: 1, y: 1 },
			ringRadius: 99,
			gate: { x: 0, y: 0 },
			area,
			areaGate: gate,
		});
		const fenceBlocksStep = must(grid.fenceBlocksStep, "fence step blocker");
		expect(fenceBlocksStep(0, 0, -1, 0)).toBe(true);
		expect(fenceBlocksStep(gate.x, gate.y, gate.x, gate.y + 1)).toBe(false);
		expect(fenceBlocksStep(0, 0, 1, 0)).toBe(false);
	});

	it("routes through the computed organic gate", () => {
		const area = fromTiles([
			{ x: 6, y: 6 },
			{ x: 7, y: 6 },
			{ x: 6, y: 7 },
			{ x: 7, y: 7 },
			{ x: 8, y: 7 },
		]);
		const gate = must(gatePlacement(area), "gate");
		expect(gate).toEqual({ x: 7, y: 7, side: "S" });
		const grid = buildColonyWalkGrid({
			tiles: [],
			anchor: VILLAGE_ANCHOR,
			ringRadius: 99,
			gate: { x: 0, y: 0 },
			area,
			areaGate: gate,
		});

		const path = must(
			findPath({ x: 6, y: 7 }, { x: 10, y: 7 }, grid),
			"organic gate path",
		);
		expect(path).toContainEqual({ x: 7, y: 8 });
		expect(path).not.toContainEqual({ x: 6, y: 8 });
	});

	it("keeps the legacy ring when no area is supplied", () => {
		const grid = buildColonyWalkGrid({
			tiles: [],
			anchor: { x: 0, y: 0 },
			ringRadius: 2,
			gate: { x: 0, y: 2 },
		});
		expect(grid.fenceBlocksStep).toBeUndefined();
		expect(grid.isBlocked(2, 0)).toBe(true);
		expect(grid.isBlocked(0, 2)).toBe(false);
	});

	it("invalidates a cached mid-route path when expansion moves the gate", () => {
		const colony = ensureGlobalColony(db);
		flattenWorld(colony._id);
		const originalArea = [
			{ x: 6, y: 6 },
			{ x: 7, y: 6 },
			{ x: 6, y: 7 },
			{ x: 7, y: 7 },
		];
		db.update(colonies)
			.set({ claimedTiles: originalArea })
			.where(eq(colonies._id, colony._id))
			.run();
		for (const building of db
			.select()
			.from(buildings)
			.where(eq(buildings.colonyId, colony._id))
			.all()) {
			if (building.type !== "shrine") {
				db.delete(buildings).where(eq(buildings._id, building._id)).run();
			}
		}
		const alive = db
			.select()
			.from(cats)
			.where(and(eq(cats.colonyId, colony._id), isNull(cats.deathTime)))
			.all();
		const [traveler, expander, ...rest] = alive;
		for (const cat of rest) {
			db.update(cats)
				.set({ deathTime: Date.now() })
				.where(eq(cats._id, cat._id))
				.run();
		}
		db.update(cats)
			.set({
				position: { map: "world", x: 6, y: 6 },
				destination: { map: "world", x: 10, y: 7 },
				activity: "traveling",
				currentTask: null,
			})
			.where(eq(cats._id, traveler._id))
			.run();

		advanceTime(db, 1);
		workerTick(db);

		db.update(cats)
			.set({
				position: { map: "world", x: 6, y: 7 },
				destination: { map: "world", x: 10, y: 7 },
				activity: "traveling",
				currentTask: null,
			})
			.where(eq(cats._id, traveler._id))
			.run();
		const now = Date.now();
		db.insert(jobs)
			.values({
				_id: "expand-moves-gate",
				colonyId: colony._id,
				kind: "expand_village",
				status: "active",
				requestedByType: "leader",
				requestedByPlayerId: null,
				assignedCatId: expander._id,
				baseDurationSec: 1,
				speedMultiplier: 1,
				yieldMultiplier: 1,
				clickTimeReducedSec: 0,
				createdAt: now - 2000,
				startedAt: now - 2000,
				endsAt: now - 1000,
				metadata: { target: { x: 8, y: 7 } },
			})
			.run();

		advanceTime(db, 2);
		workerTick(db);

		const moved = must(
			db.select().from(cats).where(eq(cats._id, traveler._id)).get(),
			"moved traveler",
		);
		expect(moved.position).toMatchObject({ map: "world", x: 7, y: 7 });
		expect(getColony().claimedTiles).toContainEqual({ x: 8, y: 7 });
	});
});
