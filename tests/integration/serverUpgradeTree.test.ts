/**
 * Integration tests for the research / god upgrade tree wired into the tick.
 *
 * Covers god purchases (spend blessings, unlock a node), cat research accrual
 * with auto-unlock over accelerated time, planBuilding era gating, and an
 * upgrade effect (housingPerDen) observably changing the dashboard capacity.
 */

import { eq } from "drizzle-orm";
import { nanoid } from "nanoid";
import { beforeEach, describe, expect, it } from "vitest";
import { createDb, type GameDb } from "@/db/client";
import { buildings as buildingsTable, cats, colonies } from "@/db/schema";
import {
	deserializeUpgradeTreeState,
	serializeUpgradeTreeState,
	type UpgradeTreeState,
} from "@/lib/game/upgradeTree";
import {
	advanceTime,
	ensureGlobalColony,
	getGlobalDashboard,
	planBuilding,
	unlockNode,
	workerTick,
} from "@/server/game";

const SESSION = { sessionId: "session_tree_1", nickname: "Scholar" };

let db: GameDb;

beforeEach(() => {
	db = createDb(":memory:");
});

function setBlessings(colonyId: string, points: number) {
	db.update(colonies)
		.set({ globalUpgradePoints: points })
		.where(eq(colonies._id, colonyId))
		.run();
}

function setTree(colonyId: string, state: UpgradeTreeState) {
	db.update(colonies)
		.set({ upgradeTree: serializeUpgradeTreeState(state) })
		.where(eq(colonies._id, colonyId))
		.run();
}

function readTree(colonyId: string): UpgradeTreeState {
	const colony = db
		.select()
		.from(colonies)
		.where(eq(colonies._id, colonyId))
		.get();
	return deserializeUpgradeTreeState(colony?.upgradeTree ?? null);
}

function addResearchHut(colonyId: string): string {
	const id = nanoid();
	db.insert(buildingsTable)
		.values({
			_id: id,
			colonyId,
			type: "research_hut",
			level: 1,
			position: { x: 1, y: 1 },
			constructionProgress: 100,
			productionProgress: 0,
		})
		.run();
	return id;
}

function addDen(colonyId: string): string {
	const id = nanoid();
	db.insert(buildingsTable)
		.values({
			_id: id,
			colonyId,
			type: "den",
			level: 1,
			position: { x: 2, y: 2 },
			constructionProgress: 100,
			productionProgress: 0,
		})
		.run();
	return id;
}

describe("god purchases (unlockNode)", () => {
	it("spends blessings and unlocks a node", () => {
		const colony = ensureGlobalColony(db);
		setBlessings(colony._id, 10);

		const result = unlockNode(db, { ...SESSION, nodeId: "research_hut" }) as {
			ok: boolean;
			remainingBlessings: number;
		};

		expect(result.ok).toBe(true);
		// research_hut costs 5.
		expect(result.remainingBlessings).toBe(5);
		expect(readTree(colony._id).ownedNodeIds).toContain("research_hut");
		expect(ensureGlobalColony(db).globalUpgradePoints).toBe(5);
	});

	it("rejects when blessings are insufficient", () => {
		const colony = ensureGlobalColony(db);
		setTree(colony._id, { ownedNodeIds: ["research_hut"], researchPoints: 0 });
		setBlessings(colony._id, 2);

		const result = unlockNode(db, { ...SESSION, nodeId: "basic_tools" }) as {
			ok: boolean;
			reason?: string;
		};

		expect(result.ok).toBe(false);
		expect(result.reason).toBe("insufficient-blessings");
		expect(readTree(colony._id).ownedNodeIds).not.toContain("basic_tools");
		// Blessings untouched on a failed purchase.
		expect(ensureGlobalColony(db).globalUpgradePoints).toBe(2);
	});

	it("rejects a node whose prerequisites are unmet", () => {
		const colony = ensureGlobalColony(db);
		setBlessings(colony._id, 100);

		const result = unlockNode(db, { ...SESSION, nodeId: "basic_tools" }) as {
			ok: boolean;
			reason?: string;
		};

		expect(result.ok).toBe(false);
		expect(result.reason).toBe("prerequisites-unmet");
		expect(ensureGlobalColony(db).globalUpgradePoints).toBe(100);
	});
});

describe("cat research accrual and auto-unlock", () => {
	it("accrues points from a staffed hut and auto-unlocks the cheapest node", () => {
		const colony = ensureGlobalColony(db);

		// The root is already researched and a hut stands; seed points just
		// below the cheapest unlockable node (basic_tools, cost 5).
		setTree(colony._id, {
			ownedNodeIds: ["research_hut"],
			researchPoints: 4.9,
		});
		const hutId = addResearchHut(colony._id);

		// Assign a living cat as the researcher.
		const researcher = db
			.select()
			.from(cats)
			.where(eq(cats.colonyId, colony._id))
			.all()[0];
		db.update(cats)
			.set({ assignedBuildingId: hutId })
			.where(eq(cats._id, researcher._id))
			.run();

		// Decouple the clocks: heavy time-scale drives research fast while
		// zeroed decay keeps the colony (and the researcher) alive.
		db.update(colonies)
			.set({ testTimeScale: 1000, testResourceDecayMultiplier: 0 })
			.where(eq(colonies._id, colony._id))
			.run();

		advanceTime(db, 20); // 20 * 1000 = 20000 research-seconds
		workerTick(db);

		const tree = readTree(colony._id);
		expect(tree.ownedNodeIds).toContain("basic_tools");
		// The cost of the unlocked node was spent from the pool.
		expect(tree.researchPoints).toBeLessThan(5);
	});

	it("does not accrue research without an assigned researcher", () => {
		const colony = ensureGlobalColony(db);
		setTree(colony._id, {
			ownedNodeIds: ["research_hut"],
			researchPoints: 0,
		});
		addResearchHut(colony._id); // hut stands but nobody is assigned

		db.update(colonies)
			.set({ testTimeScale: 1000, testResourceDecayMultiplier: 0 })
			.where(eq(colonies._id, colony._id))
			.run();

		advanceTime(db, 20);
		workerTick(db);

		expect(readTree(colony._id).researchPoints).toBe(0);
	});
});

describe("planBuilding era gating", () => {
	it("rejects research_hut until its node is owned", () => {
		ensureGlobalColony(db);
		expect(() =>
			planBuilding(db, { ...SESSION, type: "research_hut" }),
		).toThrow(/must be researched/);
	});

	it("accepts research_hut once its node is owned", () => {
		const colony = ensureGlobalColony(db);
		setTree(colony._id, { ownedNodeIds: ["research_hut"], researchPoints: 0 });

		const result = planBuilding(db, { ...SESSION, type: "research_hut" }) as {
			ok: boolean;
			jobId?: string;
		};
		expect(result.ok).toBe(true);
		expect(result.jobId).toBeTruthy();
	});
});

describe("upgrade effects applied live", () => {
	it("housingPerDen raises the dashboard housing capacity", () => {
		const colony = ensureGlobalColony(db);
		addDen(colony._id);
		addDen(colony._id);

		const dashboardBefore = getGlobalDashboard(db)!;
		const before = dashboardBefore.housing.capacity;
		// Count completed dens: Den Insulation adds +1 shelter to each.
		const completedDens = dashboardBefore.buildings.filter(
			(b: { type: string; constructionProgress: number }) =>
				b.type === "den" && b.constructionProgress >= 100,
		).length;
		expect(completedDens).toBeGreaterThanOrEqual(2);

		// Own Den Insulation (+1 shelter per den). Prereq research_hut owned.
		setTree(colony._id, {
			ownedNodeIds: ["research_hut", "den_insulation"],
			researchPoints: 0,
		});

		const after = getGlobalDashboard(db)!.housing.capacity;
		expect(after).toBe(before + completedDens);
	});
});
