import { describe, expect, it } from "vitest";

import {
	chunkKey,
	getVisibleChunks,
	visibleTileRect,
} from "@/lib/game/mapView";

const GEOM = { tileSize: 100, chunkSize: 12, originX: 0, originY: 0 };

describe("mapView", () => {
	describe("visibleTileRect", () => {
		it("covers the viewport with a 1-tile margin at scale 1", () => {
			const rect = visibleTileRect(
				{ tx: 0, ty: 0, scale: 1, width: 1200, height: 600 },
				GEOM,
			);
			expect(rect).toEqual({ minX: -1, maxX: 13, minY: -1, maxY: 7 });
		});

		it("shifts with pan offset", () => {
			const rect = visibleTileRect(
				{ tx: -1000, ty: -500, scale: 1, width: 1200, height: 600 },
				GEOM,
			);
			// content px x range [1000, 2200] -> tiles 10..22 plus margin
			expect(rect).toEqual({ minX: 9, maxX: 23, minY: 4, maxY: 12 });
		});

		it("expands tile coverage when zoomed out", () => {
			const rect = visibleTileRect(
				{ tx: 0, ty: 0, scale: 0.5, width: 1200, height: 600 },
				GEOM,
			);
			// content px x range [0, 2400] -> tiles 0..24 plus margin
			expect(rect).toEqual({ minX: -1, maxX: 25, minY: -1, maxY: 13 });
		});

		it("applies the tile origin offset", () => {
			const rect = visibleTileRect(
				{ tx: 0, ty: 0, scale: 1, width: 1200, height: 600 },
				{ ...GEOM, originX: -12, originY: -12 },
			);
			expect(rect).toEqual({ minX: -13, maxX: 1, minY: -13, maxY: -5 });
		});

		it("handles positive pan (content moved right/down)", () => {
			const rect = visibleTileRect(
				{ tx: 250, ty: 150, scale: 1, width: 1200, height: 600 },
				GEOM,
			);
			// content px x range [-250, 950] -> tiles -3..9 plus margin
			expect(rect).toEqual({ minX: -4, maxX: 10, minY: -3, maxY: 5 });
		});
	});

	describe("getVisibleChunks", () => {
		it("returns the chunks covering the visible tiles", () => {
			const chunks = getVisibleChunks(
				{ tx: 0, ty: 0, scale: 1, width: 1200, height: 600 },
				GEOM,
			);
			// tiles x -1..13 -> chunks -1..1; tiles y -1..7 -> chunks -1..0
			const keys = chunks.map(chunkKey).sort();
			expect(keys).toEqual(
				[
					{ chunkX: -1, chunkY: -1 },
					{ chunkX: 0, chunkY: -1 },
					{ chunkX: 1, chunkY: -1 },
					{ chunkX: -1, chunkY: 0 },
					{ chunkX: 0, chunkY: 0 },
					{ chunkX: 1, chunkY: 0 },
				]
					.map(chunkKey)
					.sort(),
			);
		});

		it("returns unique chunks only", () => {
			const chunks = getVisibleChunks(
				{ tx: 0, ty: 0, scale: 2, width: 600, height: 600 },
				GEOM,
			);
			const keys = chunks.map(chunkKey);
			expect(new Set(keys).size).toBe(keys.length);
		});

		it("covers negative coordinates when panned into them", () => {
			const chunks = getVisibleChunks(
				{ tx: 2400, ty: 2400, scale: 1, width: 600, height: 600 },
				GEOM,
			);
			for (const chunk of chunks) {
				expect(chunk.chunkX).toBeLessThanOrEqual(0);
				expect(chunk.chunkY).toBeLessThanOrEqual(0);
			}
			expect(chunks.length).toBeGreaterThan(0);
		});
	});

	describe("chunkKey", () => {
		it("encodes chunk coordinates uniquely", () => {
			expect(chunkKey({ chunkX: 1, chunkY: -2 })).not.toBe(
				chunkKey({ chunkX: -1, chunkY: 2 }),
			);
		});
	});
});
