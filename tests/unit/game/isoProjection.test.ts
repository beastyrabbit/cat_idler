import { describe, expect, it } from "vitest";

import {
	DEFAULT_ISO_GEOMETRY,
	elevationOffset,
	FLOOR_PX,
	type IsoGeometry,
	isoContentSize,
	isoToTile,
	tileDiamondCenter,
	tileToIso,
	visibleChunksIso,
	zIndexFor,
} from "@/lib/game/isoProjection";

const GEO: IsoGeometry = DEFAULT_ISO_GEOMETRY;

describe("isoProjection", () => {
	describe("measured Nature-pack geometry", () => {
		// These are measured from the Kenney "Isometric Nature" PNGs with
		// `magick -trim`: the ground diamond of naturePack_001_0.png is 182x115
		// at inset (19, 252) inside the 220x379 canvas. The renderer draws sprites
		// at native canvas size shifted by -diamondInsetX, so the pitch must equal
		// the true diamond width or adjacent diamonds gap/overlap. If the source
		// art is ever re-measured, update these together.
		it("matches the trimmed sprite bounds", () => {
			expect(GEO.tileWidth).toBe(182);
			expect(GEO.tileHeight).toBe(115);
			expect(GEO.imageWidth).toBe(220);
			expect(GEO.imageHeight).toBe(379);
			expect(GEO.diamondInsetX).toBe(19);
			expect(GEO.surfaceOffset).toBe(252);
		});

		it("keeps the diamond horizontally centered in the canvas", () => {
			// Left inset + diamond width + equal right inset == canvas width.
			expect(GEO.diamondInsetX * 2 + GEO.tileWidth).toBe(GEO.imageWidth);
		});
	});

	describe("tileToIso", () => {
		it("places the origin tile at the left edge of the top row", () => {
			const { left, top } = tileToIso(GEO.originX, GEO.originY, GEO);
			expect(top).toBe(GEO.surfacePadding);
			expect(left).toBeGreaterThanOrEqual(0);
		});

		it("offsets grid neighbors by half a diamond", () => {
			const base = tileToIso(0, 0, GEO);
			const east = tileToIso(1, 0, GEO);
			const south = tileToIso(0, 1, GEO);

			expect(east.left - base.left).toBe(GEO.tileWidth / 2);
			expect(east.top - base.top).toBe(GEO.tileHeight / 2);
			expect(south.left - base.left).toBe(-GEO.tileWidth / 2);
			expect(south.top - base.top).toBe(GEO.tileHeight / 2);
		});

		it("never produces negative content coordinates within the grid", () => {
			const corners = [
				[GEO.originX, GEO.originY],
				[GEO.originX + GEO.tilesX - 1, GEO.originY],
				[GEO.originX, GEO.originY + GEO.tilesY - 1],
				[GEO.originX + GEO.tilesX - 1, GEO.originY + GEO.tilesY - 1],
			];
			for (const [x, y] of corners) {
				const { left, top } = tileToIso(x, y, GEO);
				expect(left).toBeGreaterThanOrEqual(0);
				expect(top - GEO.surfacePadding).toBeGreaterThanOrEqual(0);
			}
		});
	});

	describe("isoToTile", () => {
		it("round-trips through the diamond center", () => {
			for (const [x, y] of [
				[0, 0],
				[6, 6],
				[-36, -36],
				[47, -12],
				[-3, 21],
			]) {
				const center = tileDiamondCenter(x, y, GEO);
				const tile = isoToTile(center.x, center.y, GEO);
				expect(tile.x).toBeCloseTo(x, 6);
				expect(tile.y).toBeCloseTo(y, 6);
			}
		});
	});

	describe("isoContentSize", () => {
		it("bounds every tile's image box", () => {
			const { width, height } = isoContentSize(GEO);
			const rightMost = tileToIso(
				GEO.originX + GEO.tilesX - 1,
				GEO.originY,
				GEO,
			);
			const bottomMost = tileToIso(
				GEO.originX + GEO.tilesX - 1,
				GEO.originY + GEO.tilesY - 1,
				GEO,
			);
			expect(width).toBeGreaterThanOrEqual(rightMost.left + GEO.tileWidth);
			expect(height).toBeGreaterThanOrEqual(
				bottomMost.top + GEO.imageHeight - GEO.surfaceOffset,
			);
		});
	});

	describe("zIndexFor", () => {
		it("orders back-to-front by x+y and objects above their own tile", () => {
			expect(zIndexFor(5, 5, "tile", GEO)).toBeLessThan(
				zIndexFor(5, 6, "tile", GEO),
			);
			expect(zIndexFor(5, 5, "object", GEO)).toBeGreaterThan(
				zIndexFor(5, 5, "tile", GEO),
			);
			// An object never overlaps a tile strictly in front of it.
			expect(zIndexFor(5, 5, "object", GEO)).toBeLessThan(
				zIndexFor(5, 6, "tile", GEO),
			);
		});

		it("stays positive across the whole grid", () => {
			expect(zIndexFor(GEO.originX, GEO.originY, "tile", GEO)).toBe(0);
			expect(
				zIndexFor(
					GEO.originX + GEO.tilesX - 1,
					GEO.originY + GEO.tilesY - 1,
					"object",
					GEO,
				),
			).toBeGreaterThan(0);
		});
	});

	describe("elevation", () => {
		it("raises a tile by one FLOOR_PX per floor", () => {
			expect(elevationOffset(0)).toBe(0);
			expect(elevationOffset(1)).toBe(FLOOR_PX);
			expect(elevationOffset(3)).toBe(3 * FLOOR_PX);
			// Negative heights never push a tile below floor 0.
			expect(elevationOffset(-2)).toBe(0);
		});

		it("keeps the raised back-corner tile on the content plane", () => {
			// A fully-raised tile at the far back corner (minimum top) must still
			// draw its sprite at a non-negative content Y.
			const { top } = tileToIso(GEO.originX, GEO.originY, GEO);
			const imgTop = top - GEO.surfaceOffset - elevationOffset(GEO.maxHeight);
			expect(imgTop).toBeGreaterThanOrEqual(0);
		});
	});

	describe("zIndexFor height", () => {
		it("stacks a taller floor above a shorter one on the same tile", () => {
			expect(zIndexFor(5, 5, "tile", GEO, 2)).toBeGreaterThan(
				zIndexFor(5, 5, "tile", GEO, 0),
			);
		});

		it("still lets terrain in front occlude a tall tile behind", () => {
			// A max-height tile behind must sit below a floor-0 tile one step in
			// front, so a cliff never pokes through the tile ahead of it.
			expect(zIndexFor(5, 5, "tile", GEO, GEO.maxHeight)).toBeLessThan(
				zIndexFor(5, 6, "tile", GEO, 0),
			);
		});

		it("defaults to floor 0 and matches the flat ordering", () => {
			expect(zIndexFor(5, 5, "tile", GEO)).toBe(
				zIndexFor(5, 5, "tile", GEO, 0),
			);
			expect(zIndexFor(GEO.originX, GEO.originY, "tile", GEO)).toBe(0);
		});
	});

	describe("visibleChunksIso", () => {
		it("includes the chunk under the view center", () => {
			// Center the viewport on the village anchor (world 6,6 → chunk 0,0).
			const center = tileDiamondCenter(6, 6, GEO);
			const scale = 1;
			const view = {
				tx: 400 - center.x * scale,
				ty: 300 - center.y * scale,
				scale,
				width: 800,
				height: 600,
			};
			const chunks = visibleChunksIso(view, GEO);
			expect(chunks.some((c) => c.chunkX === 0 && c.chunkY === 0)).toBe(true);
		});

		it("returns more chunks when zoomed out", () => {
			const center = tileDiamondCenter(6, 6, GEO);
			const near = visibleChunksIso(
				{
					tx: 400 - center.x,
					ty: 300 - center.y,
					scale: 1,
					width: 800,
					height: 600,
				},
				GEO,
			);
			const far = visibleChunksIso(
				{
					tx: 400 - center.x * 0.25,
					ty: 300 - center.y * 0.25,
					scale: 0.25,
					width: 800,
					height: 600,
				},
				GEO,
			);
			expect(far.length).toBeGreaterThan(near.length);
		});

		it("returns unique chunk coordinates", () => {
			const center = tileDiamondCenter(6, 6, GEO);
			const chunks = visibleChunksIso(
				{
					tx: 400 - center.x * 0.3,
					ty: 300 - center.y * 0.3,
					scale: 0.3,
					width: 1200,
					height: 800,
				},
				GEO,
			);
			const keys = new Set(chunks.map((c) => `${c.chunkX},${c.chunkY}`));
			expect(keys.size).toBe(chunks.length);
		});
	});
});
