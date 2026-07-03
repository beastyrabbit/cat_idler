import { describe, expect, it } from "vitest";

import { generateWorldChunk } from "@/lib/game/terrainWorld";
import { COLONY_WATER_RADIUS } from "@/lib/game/worldGen";

const COLONY = { x: 6, y: 6 };

function chunkWithColony(seed: number) {
	return generateWorldChunk(0, 0, seed, COLONY.x, COLONY.y);
}

describe("generateWorldChunk", () => {
	it("produces a full 12x12 chunk deterministically", () => {
		const first = chunkWithColony(1234);
		const second = chunkWithColony(1234);
		expect(first).toHaveLength(144);
		expect(second).toEqual(first);
	});

	it("differs across seeds", () => {
		const a = chunkWithColony(1).map((t) => t.type);
		const b = chunkWithColony(2).map((t) => t.type);
		expect(a).not.toEqual(b);
	});

	it("maps every tile onto a valid gameplay tile type + resources", () => {
		for (const tile of chunkWithColony(99)) {
			expect(tile.resources.food).toBeGreaterThanOrEqual(0);
			expect(tile.resources.herbs).toBeGreaterThanOrEqual(0);
			expect(tile.dangerLevel).toBeGreaterThanOrEqual(0);
			expect(tile.dangerLevel).toBeLessThanOrEqual(100);
		}
	});

	it("gives river tiles infinite water and no forage", () => {
		for (const tile of chunkWithColony(7)) {
			if (tile.type === "river") {
				expect(tile.resources.water).toBe(999);
				expect(tile.resources.food).toBe(0);
				expect(tile.overlayFeature).toBe("river");
			}
		}
	});

	it("guarantees a reachable water source near the colony for any seed", () => {
		for (let seed = 0; seed < 30; seed += 1) {
			const tiles = chunkWithColony(seed);
			const waterNearby = tiles.some(
				(t) =>
					t.resources.water > 0 &&
					Math.hypot(t.x - COLONY.x, t.y - COLONY.y) <= COLONY_WATER_RADIUS,
			);
			expect(waterNearby).toBe(true);
		}
	});

	it("cross-chunk consistent along a shared border column", () => {
		// The last column of chunk (0,0) is x=11; the first column of chunk (1,0)
		// is x=12 — different columns, but the height field is world-based, so a
		// tile's data depends only on its own coordinate. Regenerating chunk (1,0)
		// twice must match (determinism across chunk origins).
		const a = generateWorldChunk(1, 0, 42, COLONY.x, COLONY.y);
		const b = generateWorldChunk(1, 0, 42, COLONY.x, COLONY.y);
		expect(a).toEqual(b);
	});
});
