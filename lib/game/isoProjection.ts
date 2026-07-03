/**
 * Isometric projection for the world map.
 *
 * Sprites are Kenney "Isometric Nature" tiles: a 220x379 image whose ground
 * diamond (182x115, measured with `magick -trim` — not a forced 2:1 ratio) sits
 * inset at (19, 252) near the bottom of the canvas, with tall content (cliffs,
 * trees) rising above it. Sprites are drawn at native canvas size and shifted by
 * `-diamondInsetX` so the diamond lands exactly on the projected tile box; the
 * pitch is the diamond width itself, so adjacent diamonds tessellate seamlessly.
 * Terrain is
 * height-mapped: a tile on floor `f` is drawn raised by `f * FLOOR_PX` so cliff
 * faces read as solid columns. All math here is pure so the renderer components
 * stay thin.
 */

import type { ChunkCoord } from "./mapView";

/**
 * Vertical pixel rise per height floor (measured on the Nature cliff sprites:
 * a single-floor block raises the top surface ~71-72px above the flat diamond).
 */
export const FLOOR_PX = 72;

/** Highest floor index terrain quantizes to (matches terrainGen DEFAULT_MAX_HEIGHT). */
export const MAX_FLOORS = 3;

export interface IsoGeometry {
	/** Ground diamond width in content px (the tessellation pitch). */
	tileWidth: number;
	/** Ground diamond height in content px. */
	tileHeight: number;
	/** Full sprite canvas width (the diamond sits inset within this). */
	imageWidth: number;
	/** Full sprite canvas height (diamond + tall content above it). */
	imageHeight: number;
	/** X inset of the diamond's left vertex inside the sprite canvas. */
	diamondInsetX: number;
	/** Y offset of the diamond's top vertex inside the sprite canvas. */
	surfaceOffset: number;
	/** Vertical padding above the first diamond row so sprites fit. */
	surfacePadding: number;
	/** Tiles per chunk edge. */
	chunkSize: number;
	/** World tile rendered at grid position (0, 0). */
	originX: number;
	originY: number;
	/** Grid span in tiles. */
	tilesX: number;
	tilesY: number;
	/** Highest floor a tile can sit on (drives z-order banding + padding). */
	maxHeight: number;
}

/**
 * Matches the Kenney "Isometric Nature" source sprites at native resolution.
 *
 * The content plane spans a ±12 chunk window around the village chunk (0,0):
 * world tiles -144..155 on each axis (25 chunks × 12 tiles = 300). The village
 * anchor (6,6) stays near the center. See `chunkWindow` for the derived bounds.
 *
 * `surfacePadding` carries `surfaceOffset` (tall content above the diamond top)
 * plus `maxHeight * FLOOR_PX` of headroom so a fully-raised back-corner tile
 * still lands at a non-negative content Y.
 */
export const DEFAULT_ISO_GEOMETRY: IsoGeometry = {
	tileWidth: 182,
	tileHeight: 115,
	imageWidth: 220,
	imageHeight: 379,
	diamondInsetX: 19,
	surfaceOffset: 252,
	surfacePadding: 252 + MAX_FLOORS * FLOOR_PX,
	chunkSize: 12,
	originX: -144,
	originY: -144,
	tilesX: 300,
	tilesY: 300,
	maxHeight: MAX_FLOORS,
};

/** Vertical pixel offset a tile is raised by for sitting on floor `height`. */
export function elevationOffset(height: number): number {
	return Math.max(0, height) * FLOOR_PX;
}

/**
 * Renderable chunk window derived from the content-plane geometry. Chunks
 * outside this range have no content plane to draw on, so both the map culler
 * and the chunks API clamp to it (the world is generated on demand, but only
 * within these bounds — an unbounded window would let panning generate forever).
 */
export function chunkWindow(geo: IsoGeometry): { min: number; max: number } {
	return {
		min: Math.floor(geo.originX / geo.chunkSize),
		max: Math.floor((geo.originX + geo.tilesX - 1) / geo.chunkSize),
	};
}

/**
 * Top-left corner of a tile's ground-diamond bounding box in content px.
 * Sprite images are drawn at `top - surfaceOffset` so the diamond lands here.
 */
export function tileToIso(
	x: number,
	y: number,
	geo: IsoGeometry,
): { left: number; top: number } {
	const u = x - geo.originX;
	const v = y - geo.originY;
	return {
		left: ((u - v + (geo.tilesY - 1)) * geo.tileWidth) / 2,
		top: ((u + v) * geo.tileHeight) / 2 + geo.surfacePadding,
	};
}

/** Content-px center of a tile's ground diamond. */
export function tileDiamondCenter(
	x: number,
	y: number,
	geo: IsoGeometry,
): { x: number; y: number } {
	const { left, top } = tileToIso(x, y, geo);
	return { x: left + geo.tileWidth / 2, y: top + geo.tileHeight / 2 };
}

/** Inverse projection: content px → fractional world tile coordinates. */
export function isoToTile(
	px: number,
	py: number,
	geo: IsoGeometry,
): { x: number; y: number } {
	const a = px / (geo.tileWidth / 2) - (geo.tilesY - 1) - 1; // u - v
	const b = (py - geo.surfacePadding) / (geo.tileHeight / 2) - 1; // u + v
	return {
		x: (a + b) / 2 + geo.originX,
		y: (b - a) / 2 + geo.originY,
	};
}

/** Total content-plane size for the renderable grid. */
export function isoContentSize(geo: IsoGeometry): {
	width: number;
	height: number;
} {
	const diagonal = geo.tilesX + geo.tilesY - 2;
	return {
		width: (diagonal * geo.tileWidth) / 2 + geo.tileWidth,
		height:
			(diagonal * geo.tileHeight) / 2 +
			geo.surfacePadding +
			(geo.imageHeight - geo.surfaceOffset),
	};
}

/**
 * Painter's-order z-index. Depth (x+y) dominates so terrain in front always
 * occludes terrain behind; within a depth band, a taller floor stacks above a
 * shorter one, and objects (buildings, cats, decor) take the odd slot just
 * above their own tile+floor. `height` is the tile's floor level (default 0).
 */
export function zIndexFor(
	x: number,
	y: number,
	layer: "tile" | "object",
	geo: IsoGeometry,
	height = 0,
): number {
	const depth = x - geo.originX + (y - geo.originY);
	const floor = Math.max(0, Math.min(geo.maxHeight, height));
	return (
		(depth * (geo.maxHeight + 1) + floor) * 2 + (layer === "object" ? 1 : 0)
	);
}

/**
 * Chunks whose tiles may intersect the viewport. Inverts the projection at
 * the viewport corners and pads for tall sprites that rise above their
 * ground diamond.
 */
export function visibleChunksIso(
	view: {
		tx: number;
		ty: number;
		scale: number;
		width: number;
		height: number;
	},
	geo: IsoGeometry,
): ChunkCoord[] {
	const corners: Array<[number, number]> = [
		[0, 0],
		[view.width, 0],
		[0, view.height],
		[view.width, view.height],
	];

	let minX = Number.POSITIVE_INFINITY;
	let maxX = Number.NEGATIVE_INFINITY;
	let minY = Number.POSITIVE_INFINITY;
	let maxY = Number.NEGATIVE_INFINITY;

	for (const [sx, sy] of corners) {
		const px = (sx - view.tx) / view.scale;
		const py = (sy - view.ty) / view.scale;
		const tile = isoToTile(px, py, geo);
		minX = Math.min(minX, tile.x);
		maxX = Math.max(maxX, tile.x);
		minY = Math.min(minY, tile.y);
		maxY = Math.max(maxY, tile.y);
	}

	// Pad one tile all around, plus extra depth at the bottom edge so tall
	// sprites (up to surfaceOffset px above their diamond, plus a fully-raised
	// column) are not culled while their ground tile is just below the viewport.
	const tallPad = Math.ceil(
		(geo.surfaceOffset + geo.maxHeight * FLOOR_PX) / (geo.tileHeight / 2),
	);
	const lo = { x: Math.floor(minX) - 1, y: Math.floor(minY) - 1 };
	const hi = {
		x: Math.ceil(maxX) + 1 + tallPad,
		y: Math.ceil(maxY) + 1 + tallPad,
	};

	const chunks: ChunkCoord[] = [];
	const minChunkX = Math.floor(lo.x / geo.chunkSize);
	const maxChunkX = Math.floor(hi.x / geo.chunkSize);
	const minChunkY = Math.floor(lo.y / geo.chunkSize);
	const maxChunkY = Math.floor(hi.y / geo.chunkSize);
	for (let cy = minChunkY; cy <= maxChunkY; cy++) {
		for (let cx = minChunkX; cx <= maxChunkX; cx++) {
			chunks.push({ chunkX: cx, chunkY: cy });
		}
	}
	return chunks;
}
