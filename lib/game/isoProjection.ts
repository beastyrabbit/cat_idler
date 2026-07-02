/**
 * Isometric (2:1 diamond) projection for the world map.
 *
 * Sprites are Kenney "Isometric Miniature" tiles: a 256x512 image whose
 * ground diamond (256x128) sits near the bottom of the canvas, with tall
 * content (trees, walls) rising above it. All math here is pure so the
 * renderer components stay thin.
 */

import type { ChunkCoord } from "./mapView";

export interface IsoGeometry {
	/** Ground diamond width in content px. */
	tileWidth: number;
	/** Ground diamond height in content px (tileWidth / 2). */
	tileHeight: number;
	/** Full sprite canvas height (diamond + tall content above it). */
	imageHeight: number;
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
}

/** Matches the Kenney source sprites at native resolution. */
export const DEFAULT_ISO_GEOMETRY: IsoGeometry = {
	tileWidth: 256,
	tileHeight: 128,
	imageHeight: 512,
	surfaceOffset: 368,
	surfacePadding: 368,
	chunkSize: 12,
	originX: -36,
	originY: -36,
	tilesX: 84,
	tilesY: 84,
};

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
 * Painter's-order z-index. Tiles take even slots by depth (x+y); objects
 * (buildings, cats, decor) take the odd slot just above their own tile so
 * terrain in front still occludes them.
 */
export function zIndexFor(
	x: number,
	y: number,
	layer: "tile" | "object",
	geo: IsoGeometry,
): number {
	const depth = x - geo.originX + (y - geo.originY);
	return depth * 2 + (layer === "object" ? 1 : 0);
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
	// sprites (up to surfaceOffset px above their diamond) are not culled
	// while their ground tile is just below the viewport.
	const tallPad = Math.ceil(geo.surfaceOffset / (geo.tileHeight / 2));
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
