import { describe, expect, it } from "vitest";
import {
	CROWDING_PER_TILE,
	expandVillage,
	FENCE_DIR,
	FREE_TILE_FLOOR,
	fenceBlocksMove,
	fenceEdgeBetween,
	fenceMaskAt,
	fencePerimeter,
	fromTiles,
	gatePlacement,
	isInsideVillage,
	type Pos,
	perimeterBlocks,
	perimeterLength,
	type Side,
	shouldExpand,
	toTiles,
	type VillageArea,
} from "@/lib/game/villageArea";

// --- shape fixtures ---------------------------------------------------------

/** A solid w×h rectangle anchored at (0,0). */
function rect(w: number, h: number): Pos[] {
	const out: Pos[] = [];
	for (let y = 0; y < h; y++) {
		for (let x = 0; x < w; x++) out.push({ x, y });
	}
	return out;
}

// An L: a 3x3 block missing its top-right 1x1 (so it has a concave notch).
const L_SHAPE: Pos[] = [
	{ x: 0, y: 0 },
	{ x: 1, y: 0 },
	{ x: 0, y: 1 },
	{ x: 1, y: 1 },
	{ x: 2, y: 1 },
	{ x: 0, y: 2 },
	{ x: 1, y: 2 },
	{ x: 2, y: 2 },
];

// A T: a horizontal bar with a stem hanging down from the middle.
const T_SHAPE: Pos[] = [
	{ x: 0, y: 0 },
	{ x: 1, y: 0 },
	{ x: 2, y: 0 },
	{ x: 1, y: 1 },
	{ x: 1, y: 2 },
];

// A blobby, roughly-diagonal cluster.
const BLOB: Pos[] = [
	{ x: 0, y: 0 },
	{ x: 1, y: 0 },
	{ x: 1, y: 1 },
	{ x: 2, y: 1 },
	{ x: 2, y: 2 },
	{ x: 3, y: 2 },
];

/** Corner-point endpoints of a segment in grid-corner coordinates: tile (x,y)
 * spans corners (x,y)..(x+1,y+1). */
function segCorners(seg: {
	x: number;
	y: number;
	side: Side;
}): [string, string] {
	const { x, y, side } = seg;
	switch (side) {
		case "N":
			return [`${x},${y}`, `${x + 1},${y}`];
		case "S":
			return [`${x},${y + 1}`, `${x + 1},${y + 1}`];
		case "E":
			return [`${x + 1},${y}`, `${x + 1},${y + 1}`];
		case "W":
			return [`${x},${y}`, `${x},${y + 1}`];
	}
}

/** A boundary made only of closed loops iff every corner is used an even number
 * of times (each vertex of a simple rectilinear loop touches exactly 2 edges). */
function isClosedLoop(area: VillageArea): boolean {
	const counts = new Map<string, number>();
	for (const seg of fencePerimeter(area)) {
		for (const c of segCorners(seg)) {
			counts.set(c, (counts.get(c) ?? 0) + 1);
		}
	}
	return [...counts.values()].every((n) => n % 2 === 0);
}

// --- containment ------------------------------------------------------------

describe("isInsideVillage", () => {
	it("reports claimed tiles and rejects the rest", () => {
		const area = fromTiles(rect(2, 2));
		expect(isInsideVillage({ x: 0, y: 0 }, area)).toBe(true);
		expect(isInsideVillage({ x: 1, y: 1 }, area)).toBe(true);
		expect(isInsideVillage({ x: 2, y: 0 }, area)).toBe(false);
		expect(isInsideVillage({ x: -1, y: 0 }, area)).toBe(false);
	});

	it("round-trips through the tile list deterministically", () => {
		const tiles = BLOB;
		const area = fromTiles(tiles);
		// toTiles is row-major sorted, so it's stable regardless of input order.
		expect(toTiles(area)).toEqual(
			[...tiles].sort((a, b) => a.y - b.y || a.x - b.x),
		);
	});
});

// --- fence perimeter --------------------------------------------------------

describe("fenceMaskAt", () => {
	it("is 0 for a fully interior tile and full for a lone tile", () => {
		const big = fromTiles(rect(3, 3));
		expect(fenceMaskAt({ x: 1, y: 1 }, big)).toBe(0); // surrounded
		const lone = fromTiles([{ x: 5, y: 5 }]);
		expect(fenceMaskAt({ x: 5, y: 5 }, lone)).toBe(
			FENCE_DIR.N | FENCE_DIR.E | FENCE_DIR.S | FENCE_DIR.W,
		);
	});

	it("flags exactly the sides whose neighbour is unclaimed", () => {
		const area = fromTiles(rect(2, 1)); // (0,0),(1,0)
		// (0,0): N,S,W open; E is claimed.
		expect(fenceMaskAt({ x: 0, y: 0 }, area)).toBe(
			FENCE_DIR.N | FENCE_DIR.S | FENCE_DIR.W,
		);
		// (1,0): N,S,E open; W claimed.
		expect(fenceMaskAt({ x: 1, y: 0 }, area)).toBe(
			FENCE_DIR.N | FENCE_DIR.S | FENCE_DIR.E,
		);
	});

	it("returns 0 for tiles outside the area", () => {
		const area = fromTiles(rect(2, 2));
		expect(fenceMaskAt({ x: 9, y: 9 }, area)).toBe(0);
	});
});

describe("fencePerimeter", () => {
	it("maps N/S edges to the x-axis and E/W to the y-axis", () => {
		const seg = fencePerimeter(fromTiles([{ x: 0, y: 0 }]));
		for (const s of seg) {
			expect(s.axis).toBe(s.side === "N" || s.side === "S" ? "x" : "y");
		}
	});

	it("wraps a solid rectangle in exactly its outer edge count", () => {
		const area = fromTiles(rect(3, 2));
		// A 3x2 rectangle has perimeter 2*(3+2) = 10 unit edges.
		expect(fencePerimeter(area)).toHaveLength(10);
		expect(perimeterLength(area)).toBe(10);
	});

	it("is a single closed loop for every irregular shape", () => {
		for (const tiles of [rect(4, 3), L_SHAPE, T_SHAPE, BLOB, rect(1, 1)]) {
			expect(isClosedLoop(fromTiles(tiles))).toBe(true);
		}
	});

	it("emits segments in a deterministic row-major order", () => {
		const a = fencePerimeter(fromTiles(L_SHAPE));
		const b = fencePerimeter(fromTiles([...L_SHAPE].reverse()));
		expect(a).toEqual(b);
	});

	it("flags exactly one gate segment when a gate is supplied", () => {
		const area = fromTiles(rect(3, 3));
		const gate = gatePlacement(area)!;
		const segs = fencePerimeter(area, gate);
		expect(segs.filter((s) => s.gate)).toHaveLength(1);
		const g = segs.find((s) => s.gate)!;
		expect({ x: g.x, y: g.y, side: g.side }).toEqual(gate);
	});
});

// --- gate -------------------------------------------------------------------

describe("gatePlacement", () => {
	it("returns null for an empty area", () => {
		expect(gatePlacement(new Set())).toBeNull();
	});

	it("defaults to a southern edge (historical gate side)", () => {
		const area = fromTiles(rect(3, 3));
		const gate = gatePlacement(area)!;
		expect(gate.side).toBe("S");
		expect(gate.y).toBe(2); // bottom row
	});

	it("opens onto the most-worn outside corridor", () => {
		const area = fromTiles(rect(3, 3));
		// Heavy wear on the tile just north of (1,0): the gate should face north there.
		const gate = gatePlacement(area, {
			outsideWear: (o) => (o.x === 1 && o.y === -1 ? 100 : 0),
		})!;
		expect(gate).toEqual({ x: 1, y: 0, side: "N" });
	});

	it("falls back to the shrine→river axis bias", () => {
		const area = fromTiles(rect(3, 3));
		// Bias east: the gate should sit on an eastern edge (x = 2, side E).
		const gate = gatePlacement(area, { axisBias: { x: 1, y: 0 } })!;
		expect(gate.side).toBe("E");
		expect(gate.x).toBe(2);
	});

	it("is deterministic across equal-score ties", () => {
		const area = fromTiles(rect(3, 3));
		expect(gatePlacement(area)).toEqual(gatePlacement(area));
	});
});

// --- pathfinding blocking ---------------------------------------------------

describe("fence crossing / blocking", () => {
	const area = fromTiles(rect(3, 3));

	it("detects the boundary edge only on inside↔outside steps", () => {
		expect(fenceEdgeBetween({ x: 0, y: 0 }, { x: 1, y: 0 }, area)).toBeNull(); // both inside
		expect(fenceEdgeBetween({ x: 5, y: 5 }, { x: 6, y: 5 }, area)).toBeNull(); // both outside
		expect(fenceEdgeBetween({ x: 0, y: 0 }, { x: 2, y: 0 }, area)).toBeNull(); // not adjacent
		const e = fenceEdgeBetween({ x: 0, y: 0 }, { x: -1, y: 0 }, area);
		expect(e).toMatchObject({ x: 0, y: 0, side: "W" });
	});

	it("expresses the edge from the inside tile regardless of step direction", () => {
		const out = fenceEdgeBetween({ x: 0, y: 0 }, { x: -1, y: 0 }, area);
		const inn = fenceEdgeBetween({ x: -1, y: 0 }, { x: 0, y: 0 }, area);
		expect(out).toEqual(inn);
	});

	it("blocks every boundary crossing except the gate", () => {
		const gate = gatePlacement(area)!; // south edge of some bottom tile
		// Leaving through the gate edge is allowed...
		expect(
			fenceBlocksMove(
				{ x: gate.x, y: gate.y },
				{ x: gate.x, y: gate.y + 1 },
				area,
				gate,
			),
		).toBe(false);
		// ...but leaving through any other edge is blocked.
		expect(fenceBlocksMove({ x: 0, y: 0 }, { x: -1, y: 0 }, area, gate)).toBe(
			true,
		);
		// Interior and exterior moves never block.
		expect(fenceBlocksMove({ x: 0, y: 0 }, { x: 1, y: 0 }, area, gate)).toBe(
			false,
		);
	});

	it("perimeterBlocks is true for every segment but the gate", () => {
		expect(
			perimeterBlocks({ x: 0, y: 0, side: "N", axis: "x", gate: false }),
		).toBe(true);
		expect(
			perimeterBlocks({ x: 0, y: 0, side: "N", axis: "x", gate: true }),
		).toBe(false);
	});
});

// --- growth: one tile at a time --------------------------------------------

describe("expandVillage", () => {
	it("returns null when there is nothing to grow from", () => {
		expect(expandVillage(new Set())).toBeNull();
	});

	it("claims exactly one dry, adjacent tile", () => {
		const area = fromTiles(rect(2, 2));
		const next = expandVillage(area)!;
		expect(next).toBeTruthy();
		// Orthogonally adjacent to the area, not already claimed.
		expect(isInsideVillage(next, area)).toBe(false);
		const adj = [
			{ x: next.x + 1, y: next.y },
			{ x: next.x - 1, y: next.y },
			{ x: next.x, y: next.y + 1 },
			{ x: next.x, y: next.y - 1 },
		].some((p) => isInsideVillage(p, area));
		expect(adj).toBe(true);
	});

	it("never claims a water tile", () => {
		// Ring the 2x2 area so ALL orthogonal frontier tiles are water -> no growth.
		const area = fromTiles(rect(2, 2));
		const water = new Set(
			[
				{ x: -1, y: 0 },
				{ x: -1, y: 1 },
				{ x: 2, y: 0 },
				{ x: 2, y: 1 },
				{ x: 0, y: -1 },
				{ x: 1, y: -1 },
				{ x: 0, y: 2 },
				{ x: 1, y: 2 },
			].map((p) => `${p.x},${p.y}`),
		);
		expect(
			expandVillage(area, { isWater: (p) => water.has(`${p.x},${p.y}`) }),
		).toBeNull();
	});

	it("prefers filling a concave notch to keep the shape compact", () => {
		// L_SHAPE's notch is (2,0): claimed on W, S and SW — the most enclosed
		// frontier tile, so expansion should pick it over any outer spur.
		const next = expandVillage(fromTiles(L_SHAPE))!;
		expect(next).toEqual({ x: 2, y: 0 });
	});

	it("is deterministic given the same rng", () => {
		const area = fromTiles(rect(3, 3));
		const seeded = () => {
			// fixed stream
			let s = 0.42;
			return () => {
				s = (s * 9301 + 49297) % 233280;
				return s / 233280;
			};
		};
		expect(expandVillage(area, { rng: seeded() })).toEqual(
			expandVillage(area, { rng: seeded() }),
		);
	});

	it("keeps the area a single closed loop after a chain of expansions", () => {
		let area: VillageArea = fromTiles(rect(2, 2));
		let s = 0.1;
		const rng = () => {
			s = (s * 9301 + 49297) % 233280;
			return s / 233280;
		};
		for (let i = 0; i < 15; i++) {
			const next = expandVillage(area, { rng });
			expect(next).not.toBeNull();
			area = new Set([...area, `${next!.x},${next!.y}`]);
			expect(isClosedLoop(area)).toBe(true);
		}
	});
});

// --- growth trigger + constants --------------------------------------------

describe("shouldExpand", () => {
	it("expands from an empty footprint", () => {
		expect(shouldExpand(0, 0, 0)).toBe(true);
	});

	it("expands exactly when free buildable tiles fall below the floor", () => {
		// freeTiles = claimed - buildings. Floor is FREE_TILE_FLOOR.
		const claimed = 10;
		// free == FREE_TILE_FLOOR -> not yet (population low)
		expect(shouldExpand(0, claimed, claimed - FREE_TILE_FLOOR)).toBe(false);
		// free == FREE_TILE_FLOOR - 1 -> expand
		expect(shouldExpand(0, claimed, claimed - FREE_TILE_FLOOR + 1)).toBe(true);
	});

	it("expands under population crowding even with a free slot", () => {
		const claimed = 10;
		const buildings = claimed - FREE_TILE_FLOOR - 1; // plenty of free tiles
		// population exactly at the crowding line -> not yet
		expect(shouldExpand(claimed * CROWDING_PER_TILE, claimed, buildings)).toBe(
			false,
		);
		// just over the line -> expand
		expect(
			shouldExpand(claimed * CROWDING_PER_TILE + 1, claimed, buildings),
		).toBe(true);
	});

	it("has sane constants", () => {
		expect(FREE_TILE_FLOOR).toBeGreaterThanOrEqual(1);
		expect(CROWDING_PER_TILE).toBeGreaterThan(0);
	});
});
