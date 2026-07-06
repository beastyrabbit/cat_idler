import { describe, expect, it } from "vitest";

import {
	colonyToWorld,
	nextBuildingSite,
	ringCells,
	shrineWorldPosition,
	VILLAGE_ANCHOR,
	VILLAGE_MIN_RING,
	villageRadius,
	villageRingRadius,
	worldToColony,
} from "@/lib/game/villageLayout";

describe("villageLayout", () => {
	describe("coordinate mapping", () => {
		it("maps colony-local origin to the world anchor", () => {
			expect(colonyToWorld({ x: 0, y: 0 })).toEqual(VILLAGE_ANCHOR);
		});

		it("offsets local coordinates relative to the anchor", () => {
			expect(colonyToWorld({ x: 2, y: -1 })).toEqual({
				x: VILLAGE_ANCHOR.x + 2,
				y: VILLAGE_ANCHOR.y - 1,
			});
		});

		it("round-trips colony -> world -> colony", () => {
			const samples = [
				{ x: 0, y: 0 },
				{ x: 3, y: 5 },
				{ x: -4, y: 2 },
				{ x: -7, y: -7 },
			];
			for (const p of samples) {
				expect(worldToColony(colonyToWorld(p))).toEqual(p);
			}
		});

		it("places the shrine at the world anchor", () => {
			expect(shrineWorldPosition()).toEqual(VILLAGE_ANCHOR);
		});
	});

	describe("ringCells", () => {
		it("returns the shrine cell for ring 0", () => {
			expect(ringCells(0)).toEqual([{ x: 0, y: 0 }]);
		});

		it("returns 8 cells for ring 1", () => {
			const cells = ringCells(1);
			expect(cells).toHaveLength(8);
			for (const cell of cells) {
				expect(Math.max(Math.abs(cell.x), Math.abs(cell.y))).toBe(1);
			}
		});

		it("returns 8*r unique cells for ring r", () => {
			for (const r of [2, 3, 5]) {
				const cells = ringCells(r);
				expect(cells).toHaveLength(8 * r);
				const keys = new Set(cells.map((c) => `${c.x},${c.y}`));
				expect(keys.size).toBe(8 * r);
				for (const cell of cells) {
					expect(Math.max(Math.abs(cell.x), Math.abs(cell.y))).toBe(r);
				}
			}
		});
	});

	describe("nextBuildingSite", () => {
		it("never returns the shrine cell", () => {
			const site = nextBuildingSite([], 0);
			expect(site).not.toBeNull();
			expect(site).not.toEqual({ x: 0, y: 0 });
		});

		it("picks a ring-1 cell when the village is empty", () => {
			const site = nextBuildingSite([], 0.5);
			expect(site).not.toBeNull();
			expect(Math.max(Math.abs(site!.x), Math.abs(site!.y))).toBe(1);
		});

		it("is deterministic for a given roll", () => {
			expect(nextBuildingSite([], 0.3)).toEqual(nextBuildingSite([], 0.3));
		});

		it("uses the roll to select among free cells of the ring", () => {
			const first = nextBuildingSite([], 0);
			const last = nextBuildingSite([], 0.999);
			expect(first).not.toEqual(last);
		});

		it("skips occupied cells", () => {
			const occupied = ringCells(1).slice(0, 7);
			const site = nextBuildingSite(occupied, 0.9);
			expect(site).toEqual(ringCells(1)[7]);
		});

		it("moves to ring 2 once ring 1 is full", () => {
			const occupied = ringCells(1);
			const site = nextBuildingSite(occupied, 0);
			expect(site).not.toBeNull();
			expect(Math.max(Math.abs(site!.x), Math.abs(site!.y))).toBe(2);
		});

		it("returns null when all rings up to maxRing are full", () => {
			const occupied = [...ringCells(1), ...ringCells(2)];
			expect(nextBuildingSite(occupied, 0.5, 2)).toBeNull();
		});

		it("clamps degenerate rolls into range", () => {
			expect(nextBuildingSite([], 1)).not.toBeNull();
			expect(nextBuildingSite([], -0.1)).not.toBeNull();
		});

		it("skips blocked cells (e.g. water) and picks the next free one", () => {
			// Block every ring-1 cell except one; the site must land on it.
			const free = ringCells(1)[3];
			const site = nextBuildingSite(
				[],
				0.5,
				undefined,
				(cell) => !(cell.x === free.x && cell.y === free.y),
			);
			expect(site).toEqual(free);
		});

		it("spills to the next ring when a whole ring is blocked", () => {
			const blockedRing1 = new Set(ringCells(1).map((c) => `${c.x},${c.y}`));
			const site = nextBuildingSite([], 0.5, undefined, (cell) =>
				blockedRing1.has(`${cell.x},${cell.y}`),
			);
			expect(site).not.toBeNull();
			expect(Math.max(Math.abs(site!.x), Math.abs(site!.y))).toBe(2);
		});
	});

	describe("villageRingRadius", () => {
		it("never shrinks below the minimum founding ring", () => {
			expect(villageRingRadius(0)).toBe(VILLAGE_MIN_RING);
			expect(villageRingRadius(6)).toBe(VILLAGE_MIN_RING);
			expect(villageRingRadius(48)).toBe(VILLAGE_MIN_RING);
		});

		it("always sits at least one ring beyond the buildings", () => {
			for (const count of [1, 9, 25, 60, 100]) {
				expect(villageRingRadius(count)).toBeGreaterThan(villageRadius(count));
			}
		});

		it("steps outward once the inner rings fill (boundaries)", () => {
			// villageRadius(48)=3 -> fence 4; villageRadius(49)=4 -> fence 5.
			expect(villageRingRadius(48)).toBe(4);
			expect(villageRingRadius(49)).toBe(5);
			// villageRadius(80)=4 -> fence 5; villageRadius(81)=5 -> fence 6.
			expect(villageRingRadius(80)).toBe(5);
			expect(villageRingRadius(81)).toBe(6);
		});
	});

	describe("villageRadius", () => {
		it("has a minimum radius of 1", () => {
			expect(villageRadius(0)).toBe(1);
			expect(villageRadius(1)).toBe(1);
		});

		it("grows when ring capacity is exceeded (boundaries)", () => {
			expect(villageRadius(8)).toBe(1);
			expect(villageRadius(9)).toBe(2);
			expect(villageRadius(24)).toBe(2);
			expect(villageRadius(25)).toBe(3);
		});
	});
});
