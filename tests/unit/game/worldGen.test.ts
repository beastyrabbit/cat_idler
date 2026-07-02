import { describe, expect, it } from "vitest";

import {
	COLONY_SAFE_RADIUS,
	COLONY_WATER_RADIUS,
	chunkToTile,
	generateChunk,
	getColonyPosition,
	tileToChunk,
} from "@/lib/game/worldGen";

describe("worldGen", () => {
	describe("chunk coordinate mapping", () => {
		it("round-trips tile -> chunk -> tile origin", () => {
			expect(tileToChunk(0, 0)).toEqual({ chunkX: 0, chunkY: 0 });
			expect(tileToChunk(11, 11)).toEqual({ chunkX: 0, chunkY: 0 });
			expect(tileToChunk(12, 12)).toEqual({ chunkX: 1, chunkY: 1 });
			expect(tileToChunk(-1, -1)).toEqual({ chunkX: -1, chunkY: -1 });
			expect(chunkToTile(1, 1)).toEqual({ x: 12, y: 12 });
		});
	});

	describe("generateChunk", () => {
		it("produces a full 12x12 chunk deterministically", () => {
			const colony = getColonyPosition();
			const first = generateChunk(0, 0, 12345, colony.x, colony.y);
			const second = generateChunk(0, 0, 12345, colony.x, colony.y);

			expect(first).toHaveLength(144);
			expect(second).toEqual(first);
		});

		it("differs across seeds", () => {
			const colony = getColonyPosition();
			const a = generateChunk(0, 0, 1, colony.x, colony.y).map((t) => t.type);
			const b = generateChunk(0, 0, 2, colony.x, colony.y).map((t) => t.type);
			expect(a).not.toEqual(b);
		});
	});

	describe("guaranteed starting water", () => {
		it("always provides a water source near the colony for any seed", () => {
			const colony = getColonyPosition();

			for (const seed of [1, 7, 42, 1337, 99999, 1781313000000]) {
				const tiles = generateChunk(0, 0, seed, colony.x, colony.y);
				const waterNearby = tiles.some(
					(tile) =>
						tile.resources.water > 0 &&
						Math.sqrt((tile.x - colony.x) ** 2 + (tile.y - colony.y) ** 2) <=
							COLONY_WATER_RADIUS,
				);
				expect(waterNearby).toBe(true);
			}
		});

		it("keeps the forced water source outside the safe zone", () => {
			const colony = getColonyPosition();

			for (const seed of [1, 42, 1337]) {
				const tiles = generateChunk(0, 0, seed, colony.x, colony.y);
				for (const tile of tiles) {
					const dist = Math.sqrt(
						(tile.x - colony.x) ** 2 + (tile.y - colony.y) ** 2,
					);
					if (dist <= COLONY_SAFE_RADIUS) {
						expect(tile.type).not.toBe("river");
					}
				}
			}
		});
	});

	describe("colony safe zone", () => {
		it("never places rivers within the safe radius for any seed", () => {
			const colony = getColonyPosition();

			for (const seed of [1, 7, 42, 1337, 99999, 1781313000000]) {
				const tiles = generateChunk(0, 0, seed, colony.x, colony.y);
				const safeTiles = tiles.filter(
					(tile) =>
						Math.sqrt((tile.x - colony.x) ** 2 + (tile.y - colony.y) ** 2) <=
						COLONY_SAFE_RADIUS,
				);

				expect(safeTiles.length).toBeGreaterThan(0);
				for (const tile of safeTiles) {
					expect(tile.type).not.toBe("river");
					expect(tile.overlayFeature).not.toBe("river");
				}
			}
		});

		it("still allows rivers just outside the safe radius (boundary)", () => {
			const colony = getColonyPosition();

			// At least one of these seeds must produce a river somewhere in the
			// starting 3x3 chunks outside the safe zone — rivers exist, they are
			// only excluded from the spawn area.
			let foundRiver = false;
			for (const seed of [1, 7, 42, 1337, 99999]) {
				for (let cy = -1; cy <= 1 && !foundRiver; cy++) {
					for (let cx = -1; cx <= 1 && !foundRiver; cx++) {
						const tiles = generateChunk(cx, cy, seed, colony.x, colony.y);
						foundRiver = tiles.some(
							(tile) =>
								tile.type === "river" &&
								Math.sqrt((tile.x - colony.x) ** 2 + (tile.y - colony.y) ** 2) >
									COLONY_SAFE_RADIUS,
						);
					}
				}
				if (foundRiver) break;
			}
			expect(foundRiver).toBe(true);
		});
	});
});
