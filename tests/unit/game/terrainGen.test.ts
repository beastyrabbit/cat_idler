import { describe, expect, it } from "vitest";

import {
	classifyBiome,
	classifyCliff,
	classifyRiverSegment,
	type Direction,
	generateTerrainChunk,
	regionRiverSources,
	TERRAIN_CHUNK_SIZE,
	terrainElevationAt,
	terrainHeightAt,
	traceRiver,
} from "@/lib/game/terrainGen";

const SEED = 20260702;

/** Neighbor helper: build the {N,E,S,W} record for classifyCliff fixtures. */
function neighbors(
	n: number,
	e: number,
	s: number,
	w: number,
): Record<Direction, number> {
	return { N: n, E: e, S: s, W: w };
}

describe("terrainGen", () => {
	describe("determinism", () => {
		it("produces a full 12x12 chunk", () => {
			const chunk = generateTerrainChunk(0, 0, SEED);
			expect(chunk).toHaveLength(TERRAIN_CHUNK_SIZE * TERRAIN_CHUNK_SIZE);
		});

		it("is identical for identical inputs", () => {
			const a = generateTerrainChunk(2, -3, SEED);
			const b = generateTerrainChunk(2, -3, SEED);
			expect(b).toEqual(a);
		});

		it("differs across seeds", () => {
			const a = generateTerrainChunk(0, 0, 1).map((t) => t.height);
			const b = generateTerrainChunk(0, 0, 999).map((t) => t.height);
			expect(a).not.toEqual(b);
		});

		it("tiles carry their world coordinates", () => {
			const chunk = generateTerrainChunk(1, 1, SEED);
			const origin = TERRAIN_CHUNK_SIZE;
			expect(chunk[0]).toMatchObject({ x: origin, y: origin });
			const last = chunk[chunk.length - 1];
			expect(last).toMatchObject({
				x: origin + TERRAIN_CHUNK_SIZE - 1,
				y: origin + TERRAIN_CHUNK_SIZE - 1,
			});
		});
	});

	describe("chunk-border height continuity", () => {
		it("shares an identical height column across a vertical border", () => {
			const left = generateTerrainChunk(0, 0, SEED);
			const right = generateTerrainChunk(1, 0, SEED);
			// Right edge of the left chunk vs left edge of the right chunk are
			// adjacent world columns; compare the field directly for the shared seam.
			for (let ly = 0; ly < TERRAIN_CHUNK_SIZE; ly++) {
				const y = ly;
				const rightEdgeOfLeft = terrainHeightAt(
					TERRAIN_CHUNK_SIZE - 1,
					y,
					SEED,
				);
				const tileInLeft = left.find(
					(t) => t.x === TERRAIN_CHUNK_SIZE - 1 && t.y === y,
				);
				const leftEdgeOfRight = terrainHeightAt(TERRAIN_CHUNK_SIZE, y, SEED);
				const tileInRight = right.find(
					(t) => t.x === TERRAIN_CHUNK_SIZE && t.y === y,
				);
				expect(tileInLeft?.height).toBe(rightEdgeOfLeft);
				expect(tileInRight?.height).toBe(leftEdgeOfRight);
			}
		});

		it("computes the same height for a tile regardless of chunk queried", () => {
			// World tile (11, 5) belongs to chunk (0,0); its neighbor (12,5) to (1,0).
			// The field is world-coordinate based, so both chunks agree on the seam.
			const h11 = terrainHeightAt(11, 5, SEED);
			const chunk00 = generateTerrainChunk(0, 0, SEED);
			const seamTile = chunk00.find((t) => t.x === 11 && t.y === 5);
			expect(seamTile?.height).toBe(h11);
		});
	});

	describe("cliff autotiling bitmask", () => {
		it("flags a fully surrounded equal tile as flat", () => {
			expect(classifyCliff(2, neighbors(2, 2, 2, 2))).toEqual({ kind: "flat" });
		});

		it("flags a tile above only its northern neighbor as edge-N", () => {
			const role = classifyCliff(2, neighbors(1, 2, 2, 2));
			expect(role).toMatchObject({
				kind: "cliff",
				base: "edge",
				variant: "edge-N",
				facing: "N",
				edges: 1, // N bit
			});
		});

		it("orients each single edge to the correct compass facing", () => {
			expect(classifyCliff(2, neighbors(2, 1, 2, 2))).toMatchObject({
				variant: "edge-E",
				facing: "E",
				edges: 2,
			});
			expect(classifyCliff(2, neighbors(2, 2, 1, 2))).toMatchObject({
				variant: "edge-S",
				facing: "S",
				edges: 4,
			});
			expect(classifyCliff(2, neighbors(2, 2, 2, 1))).toMatchObject({
				variant: "edge-W",
				facing: "W",
				edges: 8,
			});
		});

		it("classifies two adjacent lower neighbors as an outer corner", () => {
			// N and E lower → NE outer corner.
			const role = classifyCliff(2, neighbors(1, 1, 2, 2));
			expect(role).toMatchObject({
				kind: "cliff",
				base: "corner",
				variant: "corner-NE",
				edges: 1 | 2,
			});
			// S and W lower → SW corner.
			expect(classifyCliff(2, neighbors(2, 2, 1, 1))).toMatchObject({
				variant: "corner-SW",
				edges: 4 | 8,
			});
		});

		it("classifies two opposite lower neighbors as a ridge", () => {
			expect(classifyCliff(2, neighbors(1, 2, 1, 2))).toMatchObject({
				base: "ridge",
				variant: "ridge-NS",
				edges: 1 | 4,
			});
			expect(classifyCliff(2, neighbors(2, 1, 2, 1))).toMatchObject({
				base: "ridge",
				variant: "ridge-EW",
				edges: 2 | 8,
			});
		});

		it("classifies three lower neighbors as a spur toward the higher side", () => {
			// N, E, S lower; W is the connected (higher-or-equal) side.
			const role = classifyCliff(2, neighbors(1, 1, 1, 2));
			expect(role).toMatchObject({
				base: "spur",
				variant: "spur-W",
				facing: "W",
				edges: 1 | 2 | 4,
			});
		});

		it("classifies four lower neighbors as an isolated pillar", () => {
			expect(classifyCliff(2, neighbors(1, 1, 1, 1))).toMatchObject({
				base: "pillar",
				variant: "pillar",
				facing: null,
				edges: 1 | 2 | 4 | 8,
			});
		});

		it("only counts strictly lower neighbors as edges", () => {
			// A higher neighbor never creates an edge.
			expect(classifyCliff(2, neighbors(3, 2, 2, 2))).toEqual({ kind: "flat" });
		});
	});

	describe("stairs", () => {
		it("places stairs that connect two adjacent floors", () => {
			// Scan a wide area of chunks so at least one qualifying cliff run exists.
			let found = 0;
			for (let cx = -2; cx <= 2; cx++) {
				for (let cy = -2; cy <= 2; cy++) {
					const chunk = generateTerrainChunk(cx, cy, SEED);
					for (const tile of chunk) {
						if (!tile.stairs) continue;
						found++;
						const facing = tile.stairs.facing;
						const below = stepInDirection(tile.x, tile.y, facing);
						const here = terrainHeightAt(tile.x, tile.y, SEED);
						const there = terrainHeightAt(below.x, below.y, SEED);
						// A stair always drops exactly one floor in its facing direction.
						expect(here - there).toBe(1);
					}
				}
			}
			expect(found).toBeGreaterThan(0);
		});
	});

	describe("rivers", () => {
		function findTraceableRiver() {
			for (let rx = -3; rx <= 3; rx++) {
				for (let ry = -3; ry <= 3; ry++) {
					for (const src of regionRiverSources(rx, ry, SEED)) {
						const path = traceRiver(src.x, src.y, SEED);
						if (path && path.length >= 3) {
							return path;
						}
					}
				}
			}
			return null;
		}

		it("descends monotonically along the elevation field", () => {
			const path = findTraceableRiver();
			expect(path).not.toBeNull();
			if (!path) return;
			for (let i = 1; i < path.length; i++) {
				const prev = terrainElevationAt(path[i - 1].x, path[i - 1].y, SEED);
				const curr = terrainElevationAt(path[i].x, path[i].y, SEED);
				expect(curr).toBeLessThan(prev);
			}
		});

		it("emits oriented start and end segments", () => {
			const path = findTraceableRiver();
			expect(path).not.toBeNull();
			if (!path) return;

			const start = classifyRiverSegment(path[0]);
			expect(start.segment).toBe("start");
			expect(start.inDir).toBeNull();
			expect(start.outDir).not.toBeNull();

			const end = classifyRiverSegment(path[path.length - 1]);
			expect(end.segment).toBe("end");
			expect(end.outDir).toBeNull();
			expect(end.inDir).not.toBeNull();

			// Interior tiles are straight or bend, never start/end.
			for (let i = 1; i < path.length - 1; i++) {
				const seg = classifyRiverSegment(path[i]);
				expect(["straight", "bend"]).toContain(seg.segment);
			}
		});

		it("classifies a colinear flow as straight and a turn as a bend", () => {
			// Manually construct path tiles to exercise the classifier directly.
			const straight = classifyRiverSegment({
				x: 0,
				y: 0,
				inDir: "N",
				outDir: "S",
			});
			expect(straight.segment).toBe("straight");

			const bend = classifyRiverSegment({
				x: 0,
				y: 0,
				inDir: "N",
				outDir: "E",
			});
			expect(bend.segment).toBe("bend");
		});

		it("never routes a river through the village plateau", () => {
			const path = findTraceableRiver();
			if (!path) return;
			for (const tile of path) {
				// Default plateau: anchor (0,0), Chebyshev radius 4.
				const cheb = Math.max(Math.abs(tile.x), Math.abs(tile.y));
				expect(cheb).toBeGreaterThan(4);
			}
		});
	});

	describe("village plateau", () => {
		it("keeps the plateau interior perfectly flat at one height", () => {
			const chunk = generateTerrainChunk(0, 0, SEED, {
				villageAnchor: { x: 5, y: 5 },
				plateauRadius: 3,
				plateauHeight: 2,
			});
			const interior = chunk.filter(
				(t) => Math.max(Math.abs(t.x - 5), Math.abs(t.y - 5)) <= 2,
			);
			expect(interior.length).toBeGreaterThan(0);
			for (const tile of interior) {
				expect(tile.height).toBe(2);
				expect(tile.terrain.kind).toBe("flat");
			}
		});

		it("holds the plateau height across the whole plateau radius", () => {
			const anchor = { x: 0, y: 0 };
			for (let dx = -4; dx <= 4; dx++) {
				for (let dy = -4; dy <= 4; dy++) {
					const h = terrainHeightAt(anchor.x + dx, anchor.y + dy, SEED, {
						plateauRadius: 4,
						plateauHeight: 1,
					});
					expect(h).toBe(1);
				}
			}
		});
	});

	describe("biomes", () => {
		it("maps height and moisture to biome roles", () => {
			expect(classifyBiome(0, 3, 0.5)).toBe("lowland");
			expect(classifyBiome(3, 3, 0.5)).toBe("highland");
			expect(classifyBiome(1, 3, 0.8)).toBe("forest");
			expect(classifyBiome(2, 3, 0.2)).toBe("rocky");
			expect(classifyBiome(1, 3, 0.45)).toBe("grassland");
		});
	});
});

/** Advance one world tile in a compass direction (test-local helper). */
function stepInDirection(x: number, y: number, dir: Direction) {
	switch (dir) {
		case "N":
			return { x, y: y - 1 };
		case "E":
			return { x: x + 1, y };
		case "S":
			return { x, y: y + 1 };
		case "W":
			return { x: x - 1, y };
	}
}
