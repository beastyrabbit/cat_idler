import { describe, expect, it } from "vitest";
import type { WorldPos } from "@/lib/game/movement";
import {
	buildColonyWalkGrid,
	findPath,
	OPEN_COST,
	ROAD_COST,
	type WalkGrid,
	type WalkTile,
} from "@/lib/game/pathfinding";

/** An open plain — nothing blocks, every step costs the same. */
const OPEN_GRID: WalkGrid = {
	isBlocked: () => false,
	cost: () => OPEN_COST,
};

/** Grid whose blocked tiles are the given "x,y" set; roads optional. */
function gridFrom(
	blocked: Set<string>,
	roads: Set<string> = new Set(),
): WalkGrid {
	return {
		isBlocked: (x, y) => blocked.has(`${x},${y}`),
		cost: (x, y) => (roads.has(`${x},${y}`) ? ROAD_COST : OPEN_COST),
	};
}

function key(p: WorldPos): string {
	return `${p.x},${p.y}`;
}

function pathKeys(path: WorldPos[] | null): string[] {
	return (path ?? []).map(key);
}

/** Every consecutive pair in a path is 4-adjacent (no diagonal, no jump). */
function isContiguous(path: WorldPos[]): boolean {
	for (let i = 1; i < path.length; i += 1) {
		const step =
			Math.abs(path[i].x - path[i - 1].x) + Math.abs(path[i].y - path[i - 1].y);
		if (step !== 1) {
			return false;
		}
	}
	return true;
}

describe("findPath", () => {
	it("returns the single start tile when start equals goal", () => {
		expect(findPath({ x: 3, y: 3 }, { x: 3, y: 3 }, OPEN_GRID)).toEqual([
			{ x: 3, y: 3 },
		]);
	});

	it("returns the straight x-before-y L on open ground", () => {
		// Same L the trail primitive walks: x-leg along y=6, then y-leg at x=20.
		const path = findPath({ x: 12, y: 6 }, { x: 20, y: 14 }, OPEN_GRID);
		expect(pathKeys(path)).toEqual([
			"12,6",
			"13,6",
			"14,6",
			"15,6",
			"16,6",
			"17,6",
			"18,6",
			"19,6",
			"20,6",
			"20,7",
			"20,8",
			"20,9",
			"20,10",
			"20,11",
			"20,12",
			"20,13",
			"20,14",
		]);
	});

	it("detours around a blocked wall of water", () => {
		// A vertical river at x=15 spanning the straight route's y range, with a
		// gap left open at y=3 so the cat can slip around the top.
		const blocked = new Set<string>();
		for (let y = 4; y <= 20; y += 1) {
			blocked.add(`15,${y}`);
		}
		const path = findPath({ x: 12, y: 6 }, { x: 20, y: 8 }, gridFrom(blocked));
		expect(path).not.toBeNull();
		const nonNull = path as WorldPos[];
		expect(isContiguous(nonNull)).toBe(true);
		// Never steps onto a river tile.
		for (const tile of nonNull) {
			expect(blocked.has(key(tile))).toBe(false);
		}
		// The detour funnels through the open gap at (15,3).
		expect(pathKeys(path)).toContain("15,3");
		expect(nonNull[0]).toEqual({ x: 12, y: 6 });
		expect(nonNull[nonNull.length - 1]).toEqual({ x: 20, y: 8 });
	});

	it("routes through the one gate in a fence ring", () => {
		const anchor = { x: 6, y: 6 };
		const ringRadius = 4;
		const gate = { x: 6, y: 10 }; // south opening
		const grid = buildColonyWalkGrid({ tiles: [], anchor, ringRadius, gate });
		// From the shrine out to a far southern tile: must exit via the gate.
		const path = findPath(anchor, { x: 6, y: 16 }, grid);
		expect(path).not.toBeNull();
		const nonNull = path as WorldPos[];
		expect(isContiguous(nonNull)).toBe(true);
		expect(pathKeys(path)).toContain("6,10");
		// The only ring tile touched is the gate — the palisade holds elsewhere.
		for (const tile of nonNull) {
			const onFence =
				Math.max(Math.abs(tile.x - anchor.x), Math.abs(tile.y - anchor.y)) ===
				ringRadius;
			if (onFence) {
				expect(tile).toEqual(gate);
			}
		}
	});

	it("prefers a road when detours are otherwise equal length", () => {
		// Straight column (0,1..3) is walled. Two equal-length detours swing
		// around it: the left is open ground, the right is a paved road. Both
		// are six steps, so only the road discount can break the tie.
		const blocked = new Set<string>(["0,1", "0,2", "0,3"]);
		const roads = new Set<string>(["1,0", "1,1", "1,2", "1,3", "1,4"]);
		const path = findPath(
			{ x: 0, y: 0 },
			{ x: 0, y: 4 },
			gridFrom(blocked, roads),
			{ margin: 3 },
		);
		expect(path).not.toBeNull();
		// It swings east onto the road corridor rather than west over open ground.
		expect(pathKeys(path)).toContain("1,2");
		expect(pathKeys(path)).not.toContain("-1,2");
	});

	it("returns null when the goal is walled off, so the caller can fall back", () => {
		// Ring the goal completely in blocked tiles.
		const blocked = new Set<string>([
			"9,10",
			"11,10",
			"10,9",
			"10,11",
			"9,9",
			"11,11",
			"9,11",
			"11,9",
		]);
		const path = findPath({ x: 2, y: 2 }, { x: 10, y: 10 }, gridFrom(blocked), {
			margin: 3,
		});
		expect(path).toBeNull();
	});

	it("returns null when it exhausts the expansion budget", () => {
		// A maze big enough that a tiny budget cannot reach the goal.
		const blocked = new Set<string>();
		for (let y = 1; y <= 40; y += 2) {
			for (let x = 0; x <= 39; x += 1) {
				// Leave a single alternating gap per wall so a path *exists* but is long.
				if (x !== (y % 4 === 1 ? 39 : 0)) {
					blocked.add(`${x},${y}`);
				}
			}
		}
		const path = findPath({ x: 0, y: 0 }, { x: 39, y: 40 }, gridFrom(blocked), {
			maxExpansions: 20,
			margin: 4,
		});
		expect(path).toBeNull();
	});
});

describe("buildColonyWalkGrid", () => {
	const anchor = { x: 6, y: 6 };
	const gate = { x: 6, y: 10 };

	function tile(
		overrides: Partial<WalkTile> & { x: number; y: number },
	): WalkTile {
		return {
			type: "grass",
			overlayFeature: null,
			resources: { water: 0 },
			pathWear: 0,
			...overrides,
		};
	}

	it("blocks river tiles and resource-pool water", () => {
		const grid = buildColonyWalkGrid({
			tiles: [
				tile({ x: 0, y: 0, type: "river" }),
				tile({ x: 1, y: 0, resources: { water: 5 } }),
				tile({ x: 2, y: 0, overlayFeature: "river" }),
				tile({ x: 3, y: 0 }),
			],
			anchor,
			ringRadius: 4,
			gate,
		});
		expect(grid.isBlocked(0, 0)).toBe(true);
		expect(grid.isBlocked(1, 0)).toBe(true);
		expect(grid.isBlocked(2, 0)).toBe(true);
		expect(grid.isBlocked(3, 0)).toBe(false);
	});

	it("blocks the fence ring everywhere but the gate", () => {
		const grid = buildColonyWalkGrid({
			tiles: [],
			anchor,
			ringRadius: 4,
			gate,
		});
		expect(grid.isBlocked(10, 6)).toBe(true); // east fence tile
		expect(grid.isBlocked(6, 2)).toBe(true); // north fence tile
		expect(grid.isBlocked(gate.x, gate.y)).toBe(false); // the gate
		expect(grid.isBlocked(6, 6)).toBe(false); // interior
		expect(grid.isBlocked(6, 20)).toBe(false); // well outside
	});

	it("discounts road tiles", () => {
		const grid = buildColonyWalkGrid({
			tiles: [
				tile({ x: 0, y: 0, overlayFeature: "road_built" }),
				tile({ x: 1, y: 0, pathWear: 80 }),
				tile({ x: 2, y: 0, pathWear: 20 }),
			],
			anchor,
			ringRadius: 4,
			gate,
		});
		expect(grid.cost(0, 0)).toBe(ROAD_COST);
		expect(grid.cost(1, 0)).toBe(ROAD_COST);
		expect(grid.cost(2, 0)).toBe(OPEN_COST);
		expect(grid.cost(50, 50)).toBe(OPEN_COST); // unknown tile → open ground
	});
});
